#[macro_export]
macro_rules! impl_diesel {
    ($ty:ident, $other:ident, $diesel:ident) => {
        #[cfg(feature = "server")]
        impl diesel::expression::AsExpression<diesel::sql_types::$diesel> for $ty {
            type Expression =
                <$other as diesel::expression::AsExpression<diesel::sql_types::$diesel>>::Expression;

            fn as_expression(self) -> Self::Expression {
                let val: $other = self.into();
                diesel::expression::AsExpression::<diesel::sql_types::$diesel>::as_expression(val)
            }
        }

        #[cfg(feature = "server")]
        impl<'__a> diesel::expression::AsExpression<diesel::sql_types::$diesel> for &'__a $ty {
            type Expression =
                <&'__a $other as diesel::expression::AsExpression<diesel::sql_types::$diesel>>::Expression;

            fn as_expression(self) -> Self::Expression {
                let val: &'__a $other = std::ops::Deref::deref(self);
                diesel::expression::AsExpression::<diesel::sql_types::$diesel>::as_expression(val)
            }
        }

        #[cfg(feature = "server")]
        impl diesel::expression::AsExpression<diesel::sql_types::Nullable<diesel::sql_types::$diesel>>
            for $ty
        {
            type Expression = <$other as diesel::expression::AsExpression<
                diesel::sql_types::Nullable<diesel::sql_types::$diesel>,
            >>::Expression;

            fn as_expression(self) -> Self::Expression {
                let val: $other = self.into();
                diesel::expression::AsExpression::<
                    diesel::sql_types::Nullable<diesel::sql_types::$diesel>,
                >::as_expression(val)
            }
        }

        #[cfg(feature = "server")]
        impl<'__a> diesel::expression::AsExpression<diesel::sql_types::Nullable<diesel::sql_types::$diesel>>
            for &'__a $ty
        {
            type Expression = <&'__a $other as diesel::expression::AsExpression<
                diesel::sql_types::Nullable<diesel::sql_types::$diesel>,
            >>::Expression;

            fn as_expression(self) -> Self::Expression {
                let val: &'__a $other = std::ops::Deref::deref(self);
                diesel::expression::AsExpression::<
                    diesel::sql_types::Nullable<diesel::sql_types::$diesel>,
                >::as_expression(val)
            }
        }

        #[cfg(feature = "server")]
        impl diesel::serialize::ToSql<diesel::sql_types::$diesel, diesel::pg::Pg> for $ty {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                <$other as diesel::serialize::ToSql<diesel::sql_types::$diesel, diesel::pg::Pg>>::to_sql(
                    <$ty as std::ops::Deref>::deref(self),
                    out,
                )
            }
        }

        #[cfg(feature = "server")]
        impl diesel::deserialize::FromSql<diesel::sql_types::$diesel, diesel::pg::Pg> for $ty {
            fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
                let val = <$other as diesel::deserialize::FromSql<
                    diesel::sql_types::$diesel,
                    diesel::pg::Pg,
                >>::from_sql(bytes)?;
                $ty::try_new(val).map_err(Into::into)
            }
        }

        #[cfg(feature = "server")]
        impl diesel::deserialize::Queryable<diesel::sql_types::$diesel, diesel::pg::Pg> for $ty {
            type Row = $other;

            fn build(row: $other) -> diesel::deserialize::Result<Self> {
                $ty::try_new(row).map_err(Into::into)
            }
        }
    };
}
