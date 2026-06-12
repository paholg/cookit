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
    #[diesel_rpc(create, read, update, delete)]
    pub id: IngredientId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    #[diesel_rpc(read)]
    pub updated_at: Timestamp,
    #[diesel_rpc(create, read, update)]
    pub name: Name,
    #[diesel_rpc(create, read, update)]
    pub density_g_per_ml: Option<PositiveFloat>,
    #[diesel_rpc(create, read, update)]
    pub grocery_section: Option<GrocerySection>,
    #[diesel_rpc(read)]
    pub deleted_at: Option<Timestamp>,
    #[diesel_rpc(read)]
    pub created_at: Timestamp,
}
