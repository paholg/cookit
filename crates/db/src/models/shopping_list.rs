use {
    crate::{
        Name, Slug, Timestamp,
        id::{BookId, ShoppingListId},
        models::shopping_list_item::ShoppingListItemView,
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::shopping_lists},
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct ShoppingList {
    pub id: ShoppingListId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub slug: Slug,
    pub name: Name,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItemView>,
}
