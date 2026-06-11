use {
    crate::{conn::DbConn, session::Session},
    anyhow::Context,
    db::{
        id::{BookId, IngredientId},
        models::ingredient::{Ingredient, IngredientUpdate},
        schema::ingredients,
    },
    diesel::prelude::*,
    diesel_async::RunQueryDsl,
};

pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<Ingredient>> {
    let rows = ingredients::table
        .filter(ingredients::book_id.eq(session.book_id()))
        .order(ingredients::name.asc())
        .load(session.conn())
        .await?;

    Ok(rows)
}

/// Apply an edit to an ingredient within the session's book.
pub async fn apply(
    update: IngredientUpdate,
    id: IngredientId,
    session: &mut Session,
) -> anyhow::Result<Ingredient> {
    diesel::update(
        ingredients::table
            .filter(ingredients::id.eq(id))
            .filter(ingredients::book_id.eq(session.book_id())),
    )
    .set(update)
    .returning(Ingredient::as_returning())
    .get_result(session.conn())
    .await
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("ingredient {id:?} not found"))
}

/// Columns for creating a bare ingredient (just a name); density and grocery
/// section are filled in later via the ingredient editor.
#[derive(Insertable)]
#[diesel(table_name = ingredients)]
struct IngredientNew<'a> {
    book_id: BookId,
    name: &'a str,
}

/// Find an existing ingredient by case-insensitive name within the book, or
/// create a bare one. Returns its id.
pub(crate) async fn get_or_create(
    conn: &mut DbConn,
    book_id: BookId,
    name: &str,
) -> anyhow::Result<IngredientId> {
    let existing: Option<IngredientId> = ingredients::table
        .filter(ingredients::book_id.eq(book_id))
        .filter(lower(ingredients::name).eq(name.to_lowercase()))
        .select(ingredients::id)
        .first(conn)
        .await
        .optional()
        .context("lookup ingredient by name")?;

    if let Some(id) = existing {
        return Ok(id);
    }

    diesel::insert_into(ingredients::table)
        .values(&IngredientNew { book_id, name })
        .returning(ingredients::id)
        .get_result(conn)
        .await
        .context("insert ingredient")
}

diesel::define_sql_function! {
    /// SQL `lower()`, for case-insensitive ingredient-name matching.
    fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text;
}
