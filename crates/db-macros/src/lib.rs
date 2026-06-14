//! `#[derive(DieselRpc)]` — generate the CRUD/sync layer for a Diesel model.
//!
//! From one annotated model it emits three wire structs — `…Create`, `…Update`,
//! `…Delete` — plus the server-only `Apply` / `ListSince` / `ApplyOp` impls that
//! run them against the database. The annotated struct itself is the read/response
//! type: `Apply` returns it, `list_since` pages it, and `OperationResponse` wraps
//! it. See `db::rpc` for the traits and the cross-model `Operation` enum these
//! plug into.
//!
//! ```ignore
//! #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
//! #[cfg_attr(feature = "server", derive(HasQuery, Identifiable))]
//! #[diesel_rpc(table = ingredients)]
//! pub struct Ingredient {
//!     #[diesel_rpc(create, update, delete)] pub id: IngredientId,
//!     #[diesel_rpc(create)]                 pub book_id: BookId,
//!     pub updated_at: Timestamp,
//!     #[diesel_rpc(create, update)]         pub name: Name,
//!     // …
//! }
//! ```
//!
//! Assumptions (true for every book-scoped model): the table has a `book_id`
//! column for tenant scoping, an `updated_at` column for sync ordering, a
//! nullable `deleted_at` for soft-deletes, and an `id` primary key.

use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::{format_ident, quote},
    syn::{
        Data, DeriveInput, Fields, GenericArgument, Ident, LitStr, PathArguments, Type,
        parse_macro_input, spanned::Spanned,
    },
};

#[proc_macro_derive(DieselRpc, attributes(diesel_rpc))]
pub fn derive_diesel_rpc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Which generated structs a field belongs to.
#[derive(Default, Clone, Copy)]
struct Ops {
    create: bool,
    update: bool,
    delete: bool,
}

struct FieldInfo {
    ident: Ident,
    ty: Type,
    ops: Ops,
}

impl FieldInfo {
    fn is_id(&self) -> bool {
        self.ident == "id"
    }
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let model = &input.ident;
    let table = table_name(&input)?;
    let fields = collect_fields(&input)?;

    let create = format_ident!("{model}Create");
    let update = format_ident!("{model}Update");
    let delete = format_ident!("{model}Delete");

    let create_fields = struct_fields(&fields, |o| o.create);
    let delete_fields = struct_fields(&fields, |o| o.delete);
    let update_fields = update_struct_fields(&fields);

    let id_ty = fields
        .iter()
        .find(|f| f.is_id())
        .map(|f| &f.ty)
        .ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "`#[derive(DieselRpc)]` requires an `id` field",
            )
        })?;

    let model_str = LitStr::new(&model.to_string(), model.span());

    let apply_op_impls = apply_op_impl(&[&create, &update, &delete]);

    // Always-compiled wire structs. The server-only diesel derives live in
    // `cfg_attr` so the wasm client gets plain (de)serializable structs. The
    // read/response struct is the annotated model itself, not generated here.
    let wire_structs = quote! {
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        #[cfg_attr(feature = "server", derive(::diesel::prelude::Insertable))]
        #[cfg_attr(feature = "server", diesel(table_name = crate::schema::#table))]
        pub struct #create {
            #(#create_fields,)*
        }

        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        #[cfg_attr(feature = "server", derive(::diesel::prelude::AsChangeset))]
        #[cfg_attr(feature = "server", diesel(table_name = crate::schema::#table))]
        pub struct #update {
            pub id: #id_ty,
            #(#update_fields,)*
        }

        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        pub struct #delete {
            #(#delete_fields,)*
        }
    };

    // Server-only glue. Wrapped in a `const _` block so the `use` imports stay
    // local and the prelude brings the query DSL into scope.
    let impls = quote! {
        #[cfg(feature = "server")]
        const _: () = {
            // Targeted imports rather than `diesel::prelude::*`: the prelude
            // would pull in diesel's *sync* `RunQueryDsl`, which collides with
            // `diesel_async`'s `.load`/`.get_result`.
            use ::diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
            use ::diesel_async::RunQueryDsl;
            use ::snafu::OptionExt;

            impl ::core::convert::From<#model> for crate::rpc::OperationResponse {
                fn from(response: #model) -> Self {
                    crate::rpc::OperationResponse::#model(response)
                }
            }

            impl crate::rpc::Apply for #create {
                type Response = #model;

                async fn apply(
                    self,
                    ctx: &mut dyn crate::rpc::RpcContext,
                ) -> crate::error::Result<#model> {
                    let response = ::diesel::insert_into(crate::schema::#table::table)
                        .values(&self)
                        .returning(#model::as_returning())
                        .get_result(ctx.conn())
                        .await?;

                    Ok(response)
                }
            }

            impl crate::rpc::Apply for #update {
                type Response = #model;

                async fn apply(
                    self,
                    ctx: &mut dyn crate::rpc::RpcContext,
                ) -> crate::error::Result<#model> {
                    let id = self.id;
                    let book_id = ctx.book_id()?;

                    ::diesel::update(
                        crate::schema::#table::table
                            .filter(crate::schema::#table::id.eq(id))
                            .filter(crate::schema::#table::book_id.eq(book_id)),
                    )
                    .set(&self)
                    .returning(#model::as_returning())
                    .get_result(ctx.conn())
                    .await
                    .optional()?
                    .context(crate::error::NotFoundSnafu {
                        entity: #model_str,
                        id: format!("{id:?}"),
                    })
                }
            }

            impl crate::rpc::Apply for #delete {
                type Response = #model;

                async fn apply(
                    self,
                    ctx: &mut dyn crate::rpc::RpcContext,
                ) -> crate::error::Result<#model> {
                    let id = self.id;
                    let book_id = ctx.book_id()?;

                    ::diesel::update(
                        crate::schema::#table::table
                            .filter(crate::schema::#table::id.eq(id))
                            .filter(crate::schema::#table::book_id.eq(book_id)),
                    )
                    .set(
                        crate::schema::#table::deleted_at
                            .eq(crate::Timestamp::new(::jiff::Timestamp::now())),
                    )
                    .returning(#model::as_returning())
                    .get_result(ctx.conn())
                    .await
                    .optional()?
                    .context(crate::error::NotFoundSnafu {
                        entity: #model_str,
                        id: format!("{id:?}"),
                    })
                }
            }

            impl crate::rpc::ListSince for #model {
                async fn list_since(
                    ctx: &mut dyn crate::rpc::RpcContext,
                    since: crate::Timestamp,
                ) -> crate::error::Result<crate::rpc::ListResponse<Self>> {
                    let book_id = ctx.book_id()?;

                    let records: ::std::vec::Vec<#model> = crate::schema::#table::table
                        .filter(crate::schema::#table::book_id.eq(book_id))
                        .filter(crate::schema::#table::updated_at.gt(since))
                        .order(crate::schema::#table::updated_at.asc())
                        .limit(crate::rpc::PAGE_SIZE)
                        .select(#model::as_select())
                        .load(ctx.conn())
                        .await?;

                    let cursor = (records.len() as i64 == crate::rpc::PAGE_SIZE)
                        .then(|| records.last().map(|r| r.updated_at))
                        .flatten();

                    Ok(crate::rpc::ListResponse { records, cursor })
                }
            }

            #apply_op_impls
        };
    };

    Ok(quote! {
        #wire_structs
        #impls
    })
}

/// Reads the required `#[diesel_rpc(table = <ident>)]` on the struct.
fn table_name(input: &DeriveInput) -> syn::Result<Ident> {
    let mut table = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("diesel_rpc") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") {
                table = Some(meta.value()?.parse::<Ident>()?);
                Ok(())
            } else {
                Err(meta.error("expected `table = <name>`"))
            }
        })?;
    }

    table.ok_or_else(|| {
        syn::Error::new(
            input.span(),
            "`#[derive(DieselRpc)]` requires `#[diesel_rpc(table = <name>)]` on the struct",
        )
    })
}

fn collect_fields(input: &DeriveInput) -> syn::Result<Vec<FieldInfo>> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(DieselRpc)]` only supports structs",
        ));
    };
    let Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(DieselRpc)]` requires named fields",
        ));
    };

    named
        .named
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .clone()
                .ok_or_else(|| syn::Error::new(field.span(), "field needs a name"))?;

            Ok(FieldInfo {
                ident,
                ty: field.ty.clone(),
                ops: parse_ops(field)?,
            })
        })
        .collect()
}

/// Parses a field's `#[diesel_rpc(create, update, delete)]` membership.
fn parse_ops(field: &syn::Field) -> syn::Result<Ops> {
    let mut ops = Ops::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("diesel_rpc") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("create") {
                ops.create = true;
            } else if meta.path.is_ident("update") {
                ops.update = true;
            } else if meta.path.is_ident("delete") {
                ops.delete = true;
            } else {
                return Err(meta.error("expected one of: create, update, delete"));
            }
            Ok(())
        })?;
    }

    Ok(ops)
}

/// `pub ident: ty` for every field selected by `pick`, preserving types verbatim.
fn struct_fields(fields: &[FieldInfo], pick: fn(&Ops) -> bool) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter(|f| pick(&f.ops))
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty }
        })
        .collect()
}

/// The update struct's body (excluding the `id` selector). Non-null columns
/// become `Option<T>` (skip-or-set); nullable columns become the
/// `Option<Option<T>>` tri-state (skip / set-NULL / set-value).
fn update_struct_fields(fields: &[FieldInfo]) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter(|f| f.ops.update && !f.is_id())
        .map(|f| {
            let ident = &f.ident;

            match option_inner(&f.ty) {
                Some(inner) => quote! {
                    #[serde(
                        default,
                        skip_serializing_if = "Option::is_none",
                        with = "serde_with::rust::double_option"
                    )]
                    pub #ident: ::core::option::Option<::core::option::Option<#inner>>
                },
                None => {
                    let ty = &f.ty;
                    quote! {
                        #[serde(default, skip_serializing_if = "Option::is_none")]
                        pub #ident: ::core::option::Option<#ty>
                    }
                }
            }
        })
        .collect()
}

/// The boxed-future `ApplyOp` bridge for each op struct, delegating to its typed
/// `Apply` and erasing the response into `OperationResponse`.
fn apply_op_impl(op_structs: &[&Ident]) -> TokenStream2 {
    let impls = op_structs.iter().map(|ident| {
        quote! {
            impl crate::rpc::ApplyOp for #ident {
                fn apply_op(
                    self,
                    ctx: &mut dyn crate::rpc::RpcContext,
                ) -> ::core::pin::Pin<::std::boxed::Box<
                    dyn ::core::future::Future<
                        Output = crate::error::Result<crate::rpc::OperationResponse>,
                    > + ::core::marker::Send + '_,
                >> {
                    ::std::boxed::Box::pin(async move {
                        Ok(crate::rpc::Apply::apply(self, ctx).await?.into())
                    })
                }
            }
        }
    });

    quote! { #(#impls)* }
}

/// `Some(inner)` if `ty` is `Option<inner>`.
fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;

    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    args.args.iter().find_map(|arg| match arg {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    })
}
