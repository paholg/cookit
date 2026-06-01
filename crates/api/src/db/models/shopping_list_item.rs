#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, shopping_list::ShoppingList},
    prelude::*,
    schema::shopping_list_items,
};
use crate::{
    db::models::ingredient::Ingredient,
    id::{BookId, IngredientId, ShoppingListId, ShoppingListItemId},
};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(ShoppingList)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Ingredient)))]
pub(crate) struct ShoppingListItem {
    pub(crate) id: ShoppingListItemId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) shopping_list_id: ShoppingListId,
    pub(crate) position: i32,
    pub(crate) quantity: Option<f64>,
    pub(crate) unit_kind: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) ingredient_id: Option<IngredientId>,
    pub(crate) text: Option<String>,
    pub(crate) checked: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = shopping_list_items))]
pub(crate) struct NewShoppingListItem<'a> {
    pub(crate) book_id: BookId,
    pub(crate) shopping_list_id: ShoppingListId,
    pub(crate) position: i32,
    pub(crate) quantity: Option<f64>,
    pub(crate) unit_kind: Option<&'a str>,
    pub(crate) unit: Option<&'a str>,
    pub(crate) ingredient_id: Option<IngredientId>,
    pub(crate) text: Option<&'a str>,
}
