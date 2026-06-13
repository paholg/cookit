use {
    crate::{
        Name, PositiveFloat, Timestamp,
        grocery_section::GrocerySection,
        id::{BookId, IngredientId},
    },
    db_macros::DieselRpc,
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::ingredients},
    diesel::prelude::{Associations, HasQuery, Identifiable},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[diesel_rpc(table = ingredients)]
pub struct Ingredient {
    #[diesel_rpc(create, update, delete)]
    pub id: IngredientId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    pub updated_at: Timestamp,
    #[diesel_rpc(create, update)]
    pub name: Name,
    #[diesel_rpc(create, update)]
    pub density_g_per_ml: Option<PositiveFloat>,
    #[diesel_rpc(create, update)]
    pub grocery_section: Option<GrocerySection>,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}
