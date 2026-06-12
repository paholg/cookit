use {
    crate::{
        Timestamp,
        grocery_section::GrocerySection,
        id::{BookId, IngredientId, ShoppingListId, ShoppingListItemId},
    },
    db_macros::DieselRpc,
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, ingredient::Ingredient, shopping_list::ShoppingList},
        schema::shopping_list_items,
    },
    diesel::prelude::{Associations, HasQuery, Identifiable},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(ShoppingList)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Ingredient)))]
#[diesel_rpc(table = shopping_list_items)]
pub struct ShoppingListItem {
    #[diesel_rpc(create, read, update, delete)]
    pub id: ShoppingListItemId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    #[diesel_rpc(read)]
    pub updated_at: Timestamp,
    #[diesel_rpc(create, read)]
    pub shopping_list_id: ShoppingListId,
    #[diesel_rpc(create, read, update)]
    pub position: i32,
    #[diesel_rpc(create, read, update)]
    pub quantity: Option<f64>,
    #[diesel_rpc(create, read, update)]
    pub unit_kind: Option<String>,
    #[diesel_rpc(create, read, update)]
    pub unit: Option<String>,
    #[diesel_rpc(create, read, update)]
    pub ingredient_id: Option<IngredientId>,
    #[diesel_rpc(create, read, update)]
    pub text: Option<String>,
    #[diesel_rpc(create, read, update)]
    pub checked: bool,
    #[diesel_rpc(read)]
    pub deleted_at: Option<Timestamp>,
    #[diesel_rpc(read)]
    pub created_at: Timestamp,
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
