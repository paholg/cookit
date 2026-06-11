use {
    serde::{Deserialize, Serialize},
    strum::{Display, EnumIter, EnumString, IntoEnumIterator},
};

/// Sections of a typical grocery store, ordered roughly by store-walking flow.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[cfg_attr(
    feature = "server",
    derive(diesel::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Text))]
#[strum(ascii_case_insensitive)]
pub enum GrocerySection {
    Produce,
    Pantry,
    Dairy,
    Frozen,
    Bakery,
    Meat,
    Alcohol,
    Other,
}

impl GrocerySection {
    pub fn alphabetical_names() -> Vec<String> {
        let mut vec: Vec<String> = GrocerySection::iter().map(|gs| gs.to_string()).collect();
        vec.sort();
        vec
    }
}

#[cfg(feature = "server")]
mod diesel_impl {
    use {
        super::GrocerySection,
        diesel::{
            backend::Backend,
            deserialize::{self, FromSql},
            pg::Pg,
            serialize::{self, Output, ToSql},
            sql_types::Text,
        },
        std::str::FromStr,
    };

    impl ToSql<Text, Pg> for GrocerySection {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
            let s = self.to_string();
            ToSql::<Text, Pg>::to_sql(s.as_str(), &mut out.reborrow())
        }
    }

    impl FromSql<Text, Pg> for GrocerySection {
        fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
            let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
            GrocerySection::from_str(&s).map_err(|e| e.to_string().into())
        }
    }
}
