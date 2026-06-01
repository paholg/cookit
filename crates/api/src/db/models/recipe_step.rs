use {
    crate::{
        db::models::recipe_step_ingredient::{
            RecipeStepIngredientBuilder, RecipeStepIngredientDetail, RecipeStepIngredientError,
        },
        duration::{format_duration, parse_duration},
        id::{BookId, DraftId, RecipeId, RecipeStepId, RecipeStepIngredientTable, RecipeStepTable},
    },
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, recipe::Recipe},
    prelude::*,
    schema::recipe_steps,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Recipe)))]
pub struct RecipeStep {
    pub id: RecipeStepId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub recipe_id: RecipeId,
    pub position: i32,
    pub text: String,
    pub duration_s: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStepDetail {
    pub step: RecipeStep,
    pub ingredients: Vec<RecipeStepIngredientDetail>,
}

/// Edit-form representation of one step and its ingredient rows.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepBuilder {
    pub id: DraftId<RecipeStepTable>,
    pub instruction: String,
    /// Free-form duration text (`30s`, `1h 30m`, ...). Empty means no timer.
    pub duration_text: String,
    pub ingredients: Vec<RecipeStepIngredientBuilder>,
}

/// Validation errors for one step and its ingredient rows, keyed by `DraftId`.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepError {
    pub instruction: Option<String>,
    pub duration: Option<String>,
    pub ingredients: HashMap<DraftId<RecipeStepIngredientTable>, RecipeStepIngredientError>,
}

impl RecipeStepError {
    pub fn is_empty(&self) -> bool {
        self.instruction.is_none() && self.duration.is_none() && self.ingredients.is_empty()
    }
}

impl RecipeStepBuilder {
    /// Validates own fields, then recurses into each ingredient.
    pub fn validate(&self) -> Result<(), RecipeStepError> {
        let mut err = RecipeStepError::default();

        if self.instruction.trim().is_empty() {
            err.instruction = Some("instruction is required".to_string());
        }

        if !self.duration_text.trim().is_empty()
            && let Err(msg) = parse_duration(&self.duration_text)
        {
            err.duration = Some(msg);
        }

        for ingredient in &self.ingredients {
            if let Err(e) = ingredient.validate() {
                err.ingredients.insert(ingredient.id, e);
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}

impl From<RecipeStepDetail> for RecipeStepBuilder {
    fn from(detail: RecipeStepDetail) -> Self {
        Self {
            id: detail.step.id.into(),
            instruction: detail.step.text,
            duration_text: detail
                .step
                .duration_s
                .map(|s| format_duration(s as i64))
                .unwrap_or_default(),
            ingredients: detail.ingredients.into_iter().map(Into::into).collect(),
        }
    }
}

/// Writable columns of `recipe_steps`. `treat_none_as_null` makes a cleared
/// timer write SQL `NULL`.
#[cfg(feature = "server")]
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = recipe_steps, treat_none_as_null = true)]
pub(crate) struct RecipeStepRecord {
    pub(crate) book_id: BookId,
    pub(crate) recipe_id: RecipeId,
    pub(crate) position: i32,
    pub(crate) text: String,
    pub(crate) duration_s: Option<i32>,
}

#[cfg(feature = "server")]
impl RecipeStepBuilder {
    /// The columns to write for this step; `position` comes from list order.
    pub(crate) fn record(
        &self,
        book_id: BookId,
        recipe_id: RecipeId,
        position: i32,
    ) -> anyhow::Result<RecipeStepRecord> {
        let duration_s = match self.duration_text.trim() {
            "" => None,
            t => Some(parse_duration(t).map_err(anyhow::Error::msg)? as i32),
        };

        Ok(RecipeStepRecord {
            book_id,
            recipe_id,
            position,
            text: self.instruction.trim().to_string(),
            duration_s,
        })
    }
}
