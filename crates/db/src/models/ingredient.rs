use {
    crate::{
        Name, PositiveFloat, Timestamp,
        grocery_section::GrocerySection,
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
    pub updated_at: Timestamp,
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(AsChangeset))]
#[cfg_attr(feature = "server", diesel(table_name = ingredients))]
pub struct IngredientUpdate {
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
}
