use {
    crate::{
        grocery_section::GrocerySection,
        id::{BookId, IngredientId, ShoppingListId, ShoppingListItemId},
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, ingredient::Ingredient, shopping_list::ShoppingList},
        schema::shopping_list_items,
    },
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(ShoppingList)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Ingredient)))]
pub struct ShoppingListItem {
    pub id: ShoppingListItemId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub shopping_list_id: ShoppingListId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<String>,
    pub unit: Option<String>,
    pub ingredient_id: Option<IngredientId>,
    pub text: Option<String>,
    pub checked: bool,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub created_at: jiff::Timestamp,
}

/// One shopping-list item flattened for display: the ingredient's name and
/// grocery section are joined in so the client can group and label rows without
/// extra lookups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListItemView {
    pub id: ShoppingListItemId,
    pub quantity: Option<f64>,
    /// Unit label (e.g. `g`, `cup`), already resolved. `None` means unitless.
    pub unit: Option<String>,
    /// Present when the item came from an ingredient rather than free text.
    pub ingredient_name: Option<String>,
    /// Free-text name for manually added items.
    pub text: Option<String>,
    pub grocery_section: Option<GrocerySection>,
    pub checked: bool,
}

impl ShoppingListItemView {
    /// The name to show: the ingredient's name, falling back to free text.
    pub fn display_name(&self) -> &str {
        self.ingredient_name
            .as_deref()
            .or(self.text.as_deref())
            .unwrap_or("")
    }
}

/// Raw add-item form input. Strings are parsed on save.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListItemInput {
    pub text: String,
    pub quantity: String,
    pub unit: String,
}
