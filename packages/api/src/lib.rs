//! Shared fullstack types and server functions for CookIt.
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};
#[cfg(feature = "server")]
pub mod db;
#[cfg(feature = "server")]
pub mod ops;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum MassUnit {
    G,
    Kg,
    Mg,
    Oz,
    Lb,
}
impl MassUnit {
    /// Multiplier to canonical grams.
    pub fn grams(self) -> f64 {
        match self {
            MassUnit::G => 1.0,
            MassUnit::Kg => 1000.0,
            MassUnit::Mg => 0.001,
            MassUnit::Oz => 28.35,
            MassUnit::Lb => 453.59,
        }
    }
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum VolumeUnit {
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
impl VolumeUnit {
    /// Multiplier to canonical milliliters.
    pub fn ml(self) -> f64 {
        match self {
            VolumeUnit::Ml => 1.0,
            VolumeUnit::L => 1000.0,
            VolumeUnit::Tsp => 4.93,
            VolumeUnit::Tbsp => 14.79,
            VolumeUnit::FlOz => 29.57,
            VolumeUnit::Cup => 236.59,
            VolumeUnit::Pt => 473.18,
            VolumeUnit::Qt => 946.35,
            VolumeUnit::Gal => 3_785.41,
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
    Mass(MassUnit),
    Volume(VolumeUnit),
    Count(String),
    Custom(String),
}
impl Unit {
    /// Build a `Unit` from a kind selector and the user-typed unit text.
    /// For Mass/Volume, the text must name a known unit (case-insensitive).
    pub fn new(kind: UnitKind, text: &str) -> Result<Self, String> {
        let t = text.trim();
        match kind {
            UnitKind::Mass => MassUnit::from_str(t).map(Unit::Mass).map_err(|_| {
                format!(
                    "unknown mass unit `{t}`; known: {}",
                    MassUnit::iter()
                        .map(|u| u.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }),
            UnitKind::Volume => VolumeUnit::from_str(t).map(Unit::Volume).map_err(|_| {
                format!(
                    "unknown volume unit `{t}`; known: {}",
                    VolumeUnit::iter()
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
    /// The user-visible unit text (e.g. "lb", "cup", "medium", or "").
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
    pub name: String,
    pub source: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    pub id: i64,
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<String>,
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
    pub grocery_section: Option<String>,
    pub ignore_density: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredient {
    pub ingredient_id: i64,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit: Unit,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meal {
    pub id: i64,
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
    pub recipe_id: i64,
    pub multiplier: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NewMeal {
    pub name: String,
    pub recipes: Vec<NewMealRecipe>,
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
#[post("/api/ingredients/:id/update")]
pub async fn update_ingredient(id: i64, input: IngredientUpdate) -> Result<(), ServerFnError> {
    ops::update_ingredient(db::pool().await, id, input)
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
#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    ops::list_meals(db::pool().await)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
#[get("/api/meals/:id")]
pub async fn get_meal(id: i64) -> Result<MealDetail, ServerFnError> {
    ops::get_meal(db::pool().await, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("meal {id} not found")))
}
#[post("/api/meals")]
pub async fn create_meal(input: NewMeal) -> Result<i64, ServerFnError> {
    ops::create_meal(db::pool().await, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
#[post("/api/meals/:id/update")]
pub async fn update_meal(id: i64, input: NewMeal) -> Result<(), ServerFnError> {
    ops::update_meal(db::pool().await, id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
