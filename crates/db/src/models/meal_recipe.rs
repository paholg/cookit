use {
    crate::{
        id::{BookId, MealId, MealRecipeDraftId, MealRecipeId, RecipeId},
        models::recipe::RecipeDetail,
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, meal::Meal, recipe::Recipe},
        schema::meal_recipes,
    },
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Meal)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Recipe)))]
pub struct MealRecipe {
    pub id: MealRecipeId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub meal_id: MealId,
    pub recipe_id: RecipeId,
    pub multiplier: f64,
    pub position: i32,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealRecipeDetail {
    pub meal_recipe: MealRecipe,
    pub recipe: RecipeDetail,
}

/// Edit-form representation of one recipe within a meal. The recipe is
/// identified by its slug (which the client already knows); the multiplier is a
/// raw string so the form can hold mid-typing input.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealRecipeBuilder {
    pub id: MealRecipeDraftId,
    /// Empty means the row is unfilled and dropped on save.
    pub recipe_slug: String,
    pub multiplier: String,
}

/// Validation errors for one meal-recipe row. Empty means valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealRecipeError {
    pub multiplier: Option<String>,
}

impl MealRecipeError {
    pub fn is_empty(&self) -> bool {
        self.multiplier.is_none()
    }
}

impl MealRecipeBuilder {
    /// A row with no recipe chosen is unfilled: skipped on save, never an error.
    pub fn is_blank(&self) -> bool {
        self.recipe_slug.trim().is_empty()
    }

    /// Validates this row in isolation.
    pub fn validate(&self) -> Result<(), MealRecipeError> {
        if self.is_blank() {
            return Ok(());
        }

        let mut err = MealRecipeError::default();
        if let Err(msg) = parse_multiplier(&self.multiplier) {
            err.multiplier = Some(msg);
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}

impl From<MealRecipeDetail> for MealRecipeBuilder {
    fn from(detail: MealRecipeDetail) -> Self {
        Self {
            id: detail.meal_recipe.id.into(),
            recipe_slug: detail.recipe.recipe.slug,
            multiplier: format_mult(detail.meal_recipe.multiplier),
        }
    }
}

/// Parse the raw multiplier field. Empty defaults to `1`; anything present must
/// be a positive number.
pub fn parse_multiplier(text: &str) -> Result<f64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(1.0);
    }

    let v: f64 = t
        .parse()
        .map_err(|_| format!("`{t}` is not a valid number"))?;

    if !v.is_finite() || v <= 0.0 {
        return Err(format!("multiplier must be a positive number, got {v}"));
    }

    Ok(v)
}

fn format_mult(m: f64) -> String {
    if m.fract().abs() < f64::EPSILON {
        format!("{}", m as i64)
    } else {
        format!("{m}")
    }
}
