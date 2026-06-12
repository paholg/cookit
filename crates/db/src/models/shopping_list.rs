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
    #[diesel_rpc(create, read, update, delete)]
    pub id: ShoppingListId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    #[diesel_rpc(read)]
    pub updated_at: Timestamp,
    #[diesel_rpc(create, read)]
    pub slug: Slug,
    #[diesel_rpc(create, read, update)]
    pub name: Name,
    #[diesel_rpc(read)]
    pub deleted_at: Option<Timestamp>,
    #[diesel_rpc(read)]
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItemView>,
}
