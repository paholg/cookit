#[cfg(feature = "server")]
use crate::db::{models::book::Book, prelude::*, schema::shopping_lists};
use crate::id::{BookId, ShoppingListId};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub(crate) struct ShoppingList {
    pub(crate) id: ShoppingListId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) slug: String,
    pub(crate) name: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = shopping_lists))]
pub(crate) struct NewShoppingList<'a> {
    pub(crate) book_id: BookId,
    pub(crate) slug: &'a str,
    pub(crate) name: &'a str,
}

// /// Aggregate ingredients across every recipe in a meal into a flat list of
// /// shopping-list items. Quantities are scaled by each recipe's multiplier,
// /// then rows with the same `(ingredient_id, unit)` are merged by summing
// /// quantities. Rows with the same ingredient but different units stay separate.
// ///
// /// `sections` maps ingredient_id to the ingredient's grocery section; pass an
// /// empty map if section data isn't available.
// pub fn aggregate_from_meal(
//     detail: &MealDetail,
//     _sections: &std::collections::HashMap<IngredientId, Option<GrocerySection>>,
// ) -> Vec<NewShoppingListItem> {
//     use std::collections::HashMap;

//     // Key on (ingredient_id, unit-label). Using the unit label string means
//     // Count("egg") and Custom("egg") land in the same bucket.
//     let mut by_key: HashMap<(IngredientId, String), usize> = HashMap::new();
//     let mut out: Vec<NewShoppingListItem> = Vec::new();

//     for mr in &detail.recipes {
//         for step in &mr.recipe_detail.steps {
//             for ing in &step.ingredients {
//                 let scaled_qty = ing.quantity.map(|q| q * mr.multiplier);
//                 let unit_label = ing.unit.as_ref().map(|u| u.label()).unwrap_or_default();
//                 let key = (ing.ingredient_id, unit_label);

//                 if let Some(&idx) = by_key.get(&key) {
//                     let existing = &mut out[idx];
//                     existing.quantity = match (existing.quantity, scaled_qty) {
//                         (Some(a), Some(b)) => Some(a + b),
//                         (Some(a), None) => Some(a),
//                         (None, b) => b,
//                     };
//                 } else {
//                     out.push(NewShoppingListItem {
//                         ingredient_id: Some(ing.ingredient_id),
//                         text: None,
//                         quantity: scaled_qty,
//                         unit: ing.unit.clone(),
//                     });
//                     by_key.insert(key, out.len() - 1);
//                 }
//             }
//         }
//     }

//     out
// }
