use {
    crate::{
        grocery_section::GrocerySection,
        helpers::{Name, PositiveFloat},
        id::{BookId, IngredientId},
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::ingredients},
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Ingredient {
    pub id: IngredientId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(AsChangeset))]
#[cfg_attr(feature = "server", diesel(table_name = ingredients))]
pub struct IngredientUpdate {
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
}
