use {
    crate::{
        id::{
            BookId, IngredientId, RecipeStepId, RecipeStepIngredientDraftId, RecipeStepIngredientId,
        },
        models::ingredient::Ingredient,
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, recipe_step::RecipeStep},
        schema::recipe_step_ingredients,
    },
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(RecipeStep, foreign_key = step_id)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Ingredient)))]
pub struct RecipeStepIngredient {
    pub id: RecipeStepIngredientId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub step_id: RecipeStepId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<String>,
    pub unit: Option<String>,
    pub ingredient_id: IngredientId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredientDetail {
    pub rsi: RecipeStepIngredient,
    pub ingredient: Ingredient,
}

/// Edit-form representation of one ingredient row. Fields are raw strings so the
/// form can hold mid-typing input; they're parsed on save.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredientBuilder {
    pub id: RecipeStepIngredientDraftId,
    /// Blank means the row is unfilled and dropped on save.
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

/// Validation errors for one ingredient row. Empty means valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepIngredientError {
    pub quantity: Option<String>,
}

impl RecipeStepIngredientError {
    pub fn is_empty(&self) -> bool {
        self.quantity.is_none()
    }
}

impl RecipeStepIngredientBuilder {
    /// A nameless row is unfilled: skipped on save, never an error.
    pub fn is_blank(&self) -> bool {
        self.name.trim().is_empty()
    }

    /// Validates this row in isolation.
    pub fn validate(&self) -> Result<(), RecipeStepIngredientError> {
        if self.is_blank() {
            return Ok(());
        }

        let mut err = RecipeStepIngredientError::default();
        if let Err(msg) = parse_quantity(&self.quantity) {
            err.quantity = Some(msg);
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}

impl From<RecipeStepIngredientDetail> for RecipeStepIngredientBuilder {
    fn from(detail: RecipeStepIngredientDetail) -> Self {
        Self {
            id: detail.rsi.id.into(),
            name: detail.ingredient.name.as_ref().to_string(),
            quantity: detail.rsi.quantity.map(format_qty).unwrap_or_default(),
            unit: detail.rsi.unit.unwrap_or_default(),
        }
    }
}

/// Parse the raw quantity field. Empty is `Ok(None)` (no quantity). Anything
/// present must be a positive number.
pub fn parse_quantity(text: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }

    let v: f64 = t
        .parse()
        .map_err(|_| format!("`{t}` is not a valid number"))?;

    if !v.is_finite() || v <= 0.0 {
        return Err(format!("quantity must be a positive number, got {v}"));
    }

    Ok(Some(v))
}

fn format_qty(q: f64) -> String {
    if q.fract().abs() < f64::EPSILON {
        format!("{}", q as i64)
    } else {
        format!("{q}")
    }
}
