use {
    crate::{
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
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub slug: String,
    pub name: String,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItemView>,
}
