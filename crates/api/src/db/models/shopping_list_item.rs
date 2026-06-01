use {
    crate::{
        grocery_section::GrocerySection,
        id::{BookId, IngredientId, ShoppingListId, ShoppingListItemId},
    },
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{
    models::{
        book::Book,
        ingredient::Ingredient,
        recipe_step_ingredient::{parse_quantity, parse_unit},
        shopping_list::ShoppingList,
    },
    prelude::*,
    schema::shopping_list_items,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
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

/// Writable columns of `shopping_list_items`.
#[cfg(feature = "server")]
#[derive(Insertable)]
#[diesel(table_name = shopping_list_items)]
pub(crate) struct ShoppingListItemRecord {
    pub(crate) book_id: BookId,
    pub(crate) shopping_list_id: ShoppingListId,
    pub(crate) position: i32,
    pub(crate) quantity: Option<f64>,
    pub(crate) unit_kind: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) ingredient_id: Option<IngredientId>,
    pub(crate) text: Option<String>,
}

#[cfg(feature = "server")]
impl ShoppingListItemInput {
    /// The columns to write for a manually added item. `position` comes from the
    /// list's current length.
    pub(crate) fn record(
        &self,
        book_id: BookId,
        shopping_list_id: ShoppingListId,
        position: i32,
    ) -> anyhow::Result<ShoppingListItemRecord> {
        let text = self.text.trim();
        anyhow::ensure!(!text.is_empty(), "item name is required");

        let unit = parse_unit(&self.unit);

        Ok(ShoppingListItemRecord {
            book_id,
            shopping_list_id,
            position,
            quantity: parse_quantity(&self.quantity).map_err(anyhow::Error::msg)?,
            unit_kind: unit.as_ref().map(|u| u.kind().to_string()),
            unit: unit.as_ref().map(|u| u.label()),
            ingredient_id: None,
            text: Some(text.to_string()),
        })
    }
}
