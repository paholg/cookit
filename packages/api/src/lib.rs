//! Shared fullstack types and server functions for CookIt.
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
pub mod db;

#[cfg(feature = "server")]
pub mod ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitKind {
    Mass,
    Volume,
    Count,
    Custom,
}

impl UnitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UnitKind::Mass => "mass",
            UnitKind::Volume => "volume",
            UnitKind::Count => "count",
            UnitKind::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mass" => Some(UnitKind::Mass),
            "volume" => Some(UnitKind::Volume),
            "count" => Some(UnitKind::Count),
            "custom" => Some(UnitKind::Custom),
            _ => None,
        }
    }
}

/// Known mass units, each with its multiplier to canonical grams.
pub const MASS_UNITS: &[(&str, f64)] = &[
    ("g", 1.0),
    ("kg", 1000.0),
    ("mg", 0.001),
    ("oz", 28.349_523_125),
    ("lb", 453.592_37),
];

/// Known volume units, each with its multiplier to canonical milliliters.
pub const VOLUME_UNITS: &[(&str, f64)] = &[
    ("ml", 1.0),
    ("l", 1000.0),
    ("tsp", 4.928_921_593_75),
    ("tbsp", 14.786_764_781_25),
    ("fl oz", 29.573_529_562_5),
    ("cup", 236.588_236_5),
    ("pt", 473.176_473),
    ("qt", 946.352_946),
    ("gal", 3_785.411_784),
];

pub fn unit_names_for(kind: UnitKind) -> &'static [(&'static str, f64)] {
    match kind {
        UnitKind::Mass => MASS_UNITS,
        UnitKind::Volume => VOLUME_UNITS,
        UnitKind::Count | UnitKind::Custom => &[],
    }
}

/// Convert a user-supplied (kind, qty, unit) into the canonical storage form.
/// - Mass → grams (unit becomes "g")
/// - Volume → ml (unit becomes "ml")
/// - Count → quantity preserved, unit forced to empty
/// - Custom → quantity preserved, unit preserved (trimmed)
///
/// Returns `Err` for an unknown mass/volume unit, or for non-finite/negative quantities.
pub fn to_canonical(
    kind: UnitKind,
    quantity: f64,
    unit: &str,
) -> Result<(f64, String), String> {
    if !quantity.is_finite() || quantity < 0.0 {
        return Err(format!("quantity must be a non-negative number, got {quantity}"));
    }
    let unit_trim = unit.trim();
    match kind {
        UnitKind::Mass => MASS_UNITS
            .iter()
            .find(|(u, _)| u.eq_ignore_ascii_case(unit_trim))
            .map(|(_, factor)| (quantity * factor, "g".to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown mass unit `{unit_trim}`; known: {}",
                    MASS_UNITS.iter().map(|(u, _)| *u).collect::<Vec<_>>().join(", ")
                )
            }),
        UnitKind::Volume => VOLUME_UNITS
            .iter()
            .find(|(u, _)| u.eq_ignore_ascii_case(unit_trim))
            .map(|(_, factor)| (quantity * factor, "ml".to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown volume unit `{unit_trim}`; known: {}",
                    VOLUME_UNITS.iter().map(|(u, _)| *u).collect::<Vec<_>>().join(", ")
                )
            }),
        UnitKind::Count => Ok((quantity, String::new())),
        UnitKind::Custom => Ok((quantity, unit_trim.to_string())),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: i64,
    pub name: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    pub id: i64,
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredient {
    pub ingredient_id: i64,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit_kind: UnitKind,
    pub unit: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStep {
    pub id: i64,
    pub position: i64,
    pub instruction: String,
    pub ingredients: Vec<RecipeStepIngredient>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewStepIngredient {
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit_kind: Option<UnitKind>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewStep {
    pub instruction: String,
    pub ingredients: Vec<NewStepIngredient>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewRecipe {
    pub name: String,
    pub source: Option<String>,
    pub steps: Vec<NewStep>,
}

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    ops::list_recipes(db::pool().await)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/recipes/:id")]
pub async fn get_recipe(id: i64) -> Result<RecipeDetail, ServerFnError> {
    ops::get_recipe(db::pool().await, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("recipe {id} not found")))
}

#[get("/api/ingredients")]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    ops::list_ingredients(db::pool().await)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes")]
pub async fn create_recipe(input: NewRecipe) -> Result<i64, ServerFnError> {
    ops::create_recipe(db::pool().await, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes/:id/update")]
pub async fn update_recipe(id: i64, input: NewRecipe) -> Result<(), ServerFnError> {
    ops::update_recipe(db::pool().await, id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
