use {
    crate::{
        Name, Slug, Timestamp,
        id::{BookId, ShoppingListId},
        models::shopping_list_item::ShoppingListItemView,
    },
    db_macros::DieselRpc,
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::shopping_lists},
    diesel::prelude::{Associations, HasQuery, Identifiable},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[diesel_rpc(table = shopping_lists)]
pub struct ShoppingList {
    #[diesel_rpc(create, update, delete)]
    pub id: ShoppingListId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    pub updated_at: Timestamp,
    #[diesel_rpc(create)]
    pub slug: Slug,
    #[diesel_rpc(create, update)]
    pub name: Name,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItemView>,
}
