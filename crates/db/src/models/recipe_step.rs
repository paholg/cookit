use {
    crate::{
        Timestamp,
        duration::{format_duration, parse_duration},
        id::{BookId, RecipeId, RecipeStepDraftId, RecipeStepId, RecipeStepIngredientDraftId},
        models::recipe_step_ingredient::{
            RecipeStepIngredientBuilder, RecipeStepIngredientDetail, RecipeStepIngredientError,
        },
    },
    db_macros::DieselRpc,
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, recipe::Recipe},
        schema::recipe_steps,
    },
    diesel::prelude::{Associations, HasQuery, Identifiable},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Recipe)))]
#[diesel_rpc(table = recipe_steps)]
pub struct RecipeStep {
    #[diesel_rpc(create, update, delete)]
    pub id: RecipeStepId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    pub updated_at: Timestamp,
    #[diesel_rpc(create)]
    pub recipe_id: RecipeId,
    #[diesel_rpc(create, update)]
    pub position: i32,
    #[diesel_rpc(create, update)]
    pub text: String,
    #[diesel_rpc(create, update)]
    pub duration_s: Option<i32>,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepDetail {
    pub step: RecipeStep,
    pub ingredients: Vec<RecipeStepIngredientDetail>,
}

/// Edit-form representation of one step and its ingredient rows.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStepBuilder {
    pub id: RecipeStepDraftId,
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
    pub ingredients: HashMap<RecipeStepIngredientDraftId, RecipeStepIngredientError>,
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
