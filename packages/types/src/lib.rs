use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Mass {
    G,
    Kg,
    Mg,
    Oz,
    Lb,
}

impl Mass {
    /// Multiplier to canonical grams.
    pub fn grams(self) -> f64 {
        match self {
            Mass::G => 1.0,
            Mass::Kg => 1000.0,
            Mass::Mg => 0.001,
            Mass::Oz => 28.35,
            Mass::Lb => 453.59,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Volume {
    Ml,
    L,
    Tsp,
    Tbsp,
    #[strum(serialize = "fl oz")]
    FlOz,
    Cup,
    Pt,
    Qt,
    Gal,
}

impl Volume {
    /// Multiplier to canonical milliliters.
    pub fn ml(self) -> f64 {
        match self {
            Volume::Ml => 1.0,
            Volume::L => 1000.0,
            Volume::Tsp => 4.93,
            Volume::Tbsp => 14.79,
            Volume::FlOz => 29.57,
            Volume::Cup => 236.59,
            Volume::Pt => 473.18,
            Volume::Qt => 946.35,
            Volume::Gal => 3_785.41,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(
    name(UnitKind),
    derive(Hash, Serialize, Deserialize, Display, EnumString),
    strum(serialize_all = "lowercase", ascii_case_insensitive),
    serde(rename_all = "lowercase")
)]
pub enum Unit {
    Mass(Mass),
    Volume(Volume),
    Count(String),
    Custom(String),
}

impl Unit {
    /// Build a `Unit` from a kind selector and the user-typed unit text.
    /// For Mass/Volume, the text must name a known unit (case-insensitive).
    pub fn new(kind: UnitKind, text: &str) -> Result<Self, String> {
        let t = text.trim();
        match kind {
            UnitKind::Mass => Mass::from_str(t).map(Unit::Mass).map_err(|_| {
                format!(
                    "unknown mass unit `{t}`; known: {}",
                    Mass::iter()
                        .map(|u| u.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }),
            UnitKind::Volume => Volume::from_str(t).map(Unit::Volume).map_err(|_| {
                format!(
                    "unknown volume unit `{t}`; known: {}",
                    Volume::iter()
                        .map(|u| u.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }),
            UnitKind::Count => Ok(Unit::Count(t.to_string())),
            UnitKind::Custom => Ok(Unit::Custom(t.to_string())),
        }
    }
    pub fn kind(&self) -> UnitKind {
        self.into()
    }
    pub fn label(&self) -> String {
        match self {
            Unit::Mass(m) => m.to_string(),
            Unit::Volume(v) => v.to_string(),
            Unit::Count(s) | Unit::Custom(s) => s.clone(),
        }
    }
}
impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub source: Option<String>,
}

/// URL-safe kebab-case slug of `name`. Used as the public identifier for
/// recipes and meals. Lowercases ASCII letters/digits, replaces runs of
/// everything else with a single `-`, trims leading/trailing `-`. Non-ASCII
/// characters are treated as separators (transliteration is out of scope).
/// Falls back to `"item"` if the result is empty so we never produce an empty
/// key — that would silently collide with `''` defaults.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.extend(c.to_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "item".to_string()
    } else {
        out
    }
}

/// Sections of a typical grocery store, ordered roughly by store-walking flow
/// so that shopping lists grouped by section read top-to-bottom in aisle order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(ascii_case_insensitive)]
pub enum GrocerySection {
    Produce,
    Bakery,
    Deli,
    Meat,
    Seafood,
    Dairy,
    Frozen,
    Pantry,
    Spices,
    Condiments,
    Beverages,
    Snacks,
    Alcohol,
    Household,
}

impl GrocerySection {
    pub fn alphabetical_names() -> Vec<String> {
        let mut vec: Vec<String> = GrocerySection::iter().map(|gs| gs.to_string()).collect();
        vec.sort();
        vec
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    pub id: i64,
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<GrocerySection>,
    pub ignore_density: bool,
}

impl Ingredient {
    /// True if the ingredient needs the user's attention — missing a grocery
    /// section, or missing a density that hasn't been explicitly ignored.
    pub fn is_incomplete(&self) -> bool {
        self.grocery_section.is_none() || (self.density_g_per_ml.is_none() && !self.ignore_density)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IngredientUpdate {
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<GrocerySection>,
    pub ignore_density: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredient {
    pub id: i64,
    pub ingredient_id: i64,
    pub ingredient_name: String,
    pub quantity: Option<f64>,
    pub unit: Option<Unit>,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepInstruction {
    pub id: i64,
    pub position: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStep {
    pub id: i64,
    pub position: i64,
    pub instructions: Vec<StepInstruction>,
    pub ingredients: Vec<RecipeStepIngredient>,
    /// Optional countdown timer length. Steps without a duration don't show
    /// the start-timer button in `RecipeView`.
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewStepIngredient {
    pub ingredient_name: String,
    pub quantity: Option<f64>,
    /// `None` means the ingredient has no unit at all (stored as null in the
    /// DB); `Some(kind)` combines with `unit` to build a `Unit` value.
    pub unit_kind: Option<UnitKind>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewStep {
    pub instructions: Vec<String>,
    pub ingredients: Vec<NewStepIngredient>,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewRecipe {
    pub name: String,
    pub source: Option<String>,
    pub steps: Vec<NewStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meal {
    pub id: i64,
    pub key: String,
    /// `None` for meals stored in the browser's localStorage (unauthenticated
    /// users); `Some(uid)` for meals owned by a user in the database.
    pub user_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealRecipe {
    pub multiplier: f64,
    pub position: i64,
    pub recipe: RecipeDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealDetail {
    pub meal: Meal,
    pub recipes: Vec<MealRecipe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewMealRecipe {
    pub recipe_key: String,
    pub multiplier: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewMeal {
    pub name: String,
    pub recipes: Vec<NewMealRecipe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingList {
    pub id: i64,
    /// `None` for lists stored in the browser's localStorage; `Some(uid)` for
    /// lists owned by a user in the database. Mirrors `Meal::user_id`.
    pub user_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListItem {
    pub id: i64,
    pub name: String,
    pub grocery_section: Option<GrocerySection>,
    pub quantity: Option<f64>,
    pub unit: Option<Unit>,
    pub checked: bool,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewShoppingListItem {
    pub name: String,
    pub grocery_section: Option<GrocerySection>,
    pub quantity: Option<f64>,
    pub unit: Option<Unit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewShoppingList {
    pub name: String,
    pub items: Vec<NewShoppingListItem>,
}

/// Aggregate every ingredient across every recipe in a meal into a flat list
/// of shopping-list rows. Quantities are scaled by each recipe's multiplier,
/// then rows with the same `(name, unit)` are merged by summing quantities.
/// Rows with the same name but different units stay separate — the UI joins
/// them inline (e.g. `"3 lb, 2 cup flour"`).
///
/// `sections` maps `ingredient_id` to the ingredient's grocery section; pass
/// an empty map if section data isn't available (everything renders under
/// "Other").
pub fn aggregate_from_meal(
    detail: &MealDetail,
    sections: &std::collections::HashMap<i64, Option<GrocerySection>>,
) -> Vec<NewShoppingListItem> {
    use std::collections::HashMap;

    // Key on (lowercase name, unit-label-or-empty). Keying on the label
    // string keeps Count("egg") and Custom("egg") in the same bucket — the
    // display string is what the shopper sees, so that's what should merge.
    let mut by_key: HashMap<(String, String), usize> = HashMap::new();
    let mut out: Vec<NewShoppingListItem> = Vec::new();

    for mr in &detail.recipes {
        for step in &mr.recipe.steps {
            for ing in &step.ingredients {
                let scaled_qty = ing.quantity.map(|q| q * mr.multiplier);
                let unit_label = ing.unit.as_ref().map(|u| u.label()).unwrap_or_default();
                let key = (ing.ingredient_name.to_lowercase(), unit_label);

                if let Some(&idx) = by_key.get(&key) {
                    let existing = &mut out[idx];
                    existing.quantity = match (existing.quantity, scaled_qty) {
                        (Some(a), Some(b)) => Some(a + b),
                        (Some(a), None) => Some(a),
                        (None, b) => b,
                    };
                } else {
                    let section = sections.get(&ing.ingredient_id).cloned().flatten();
                    out.push(NewShoppingListItem {
                        name: ing.ingredient_name.clone(),
                        grocery_section: section,
                        quantity: scaled_qty,
                        unit: ing.unit.clone(),
                    });
                    by_key.insert(key, out.len() - 1);
                }
            }
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_admin: bool,
}

/// Public identity of a user shown in the dev-only "log in as" selector.
/// Only used in builds with the `dev-auth` feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevUser {
    pub id: i64,
    pub name: String,
    pub is_admin: bool,
}

#[cfg(test)]
mod aggregate_tests {
    use super::*;
    use std::collections::HashMap;

    fn ing(id: i64, name: &str, qty: Option<f64>, unit: Option<Unit>) -> RecipeStepIngredient {
        RecipeStepIngredient {
            id: 0,
            ingredient_id: id,
            ingredient_name: name.into(),
            quantity: qty,
            unit,
            position: 0,
        }
    }

    fn meal(recipes: Vec<(f64, Vec<RecipeStepIngredient>)>) -> MealDetail {
        let recipes = recipes
            .into_iter()
            .enumerate()
            .map(|(i, (mult, ings))| MealRecipe {
                multiplier: mult,
                position: i as i64,
                recipe: RecipeDetail {
                    recipe: Recipe {
                        id: i as i64,
                        key: format!("r{i}"),
                        name: format!("r{i}"),
                        source: None,
                    },
                    steps: vec![RecipeStep {
                        id: 0,
                        position: 0,
                        instructions: vec![],
                        ingredients: ings,
                        duration_seconds: None,
                    }],
                },
            })
            .collect();
        MealDetail {
            meal: Meal {
                id: 1,
                key: "m".into(),
                user_id: None,
                name: "m".into(),
            },
            recipes,
        }
    }

    #[test]
    fn slugify_basics() {
        assert_eq!(slugify("Black Beans"), "black-beans");
        assert_eq!(slugify("  Chicken & Waffles  "), "chicken-waffles");
        assert_eq!(slugify("Mom's Chili (Spicy!)"), "mom-s-chili-spicy");
        assert_eq!(slugify("---"), "item");
        assert_eq!(slugify(""), "item");
        assert_eq!(slugify("café"), "caf");
    }

    #[test]
    fn same_unit_sums_and_scales_by_multiplier() {
        let d = meal(vec![
            (
                2.0,
                vec![ing(1, "flour", Some(1.0), Some(Unit::Mass(Mass::Lb)))],
            ),
            (
                1.0,
                vec![ing(1, "Flour", Some(1.0), Some(Unit::Mass(Mass::Lb)))],
            ),
        ]);
        let out = aggregate_from_meal(&d, &HashMap::new());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "flour");
        assert_eq!(out[0].quantity, Some(3.0));
    }

    #[test]
    fn different_units_stay_separate() {
        let d = meal(vec![(
            1.0,
            vec![
                ing(1, "flour", Some(3.0), Some(Unit::Mass(Mass::Lb))),
                ing(1, "flour", Some(2.0), Some(Unit::Volume(Volume::Cup))),
            ],
        )]);
        let out = aggregate_from_meal(&d, &HashMap::new());
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|i| i.quantity == Some(3.0)));
        assert!(out.iter().any(|i| i.quantity == Some(2.0)));
    }

    #[test]
    fn section_is_looked_up_from_map() {
        let d = meal(vec![(
            1.0,
            vec![
                ing(7, "apples", Some(2.0), None),
                ing(9, "nutmeg", Some(1.0), None),
            ],
        )]);
        let mut sections = HashMap::new();
        sections.insert(7, Some(GrocerySection::Produce));
        sections.insert(9, None);
        let out = aggregate_from_meal(&d, &sections);
        let apples = out.iter().find(|i| i.name == "apples").unwrap();
        let nutmeg = out.iter().find(|i| i.name == "nutmeg").unwrap();
        assert_eq!(apples.grocery_section, Some(GrocerySection::Produce));
        assert_eq!(nutmeg.grocery_section, None);
    }
}
