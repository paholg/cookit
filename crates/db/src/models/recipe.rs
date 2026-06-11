use {
    crate::{
        helpers::Name,
        id::{BookId, RecipeDraftId, RecipeId, RecipeStepDraftId},
        models::recipe_step::{RecipeStepBuilder, RecipeStepDetail, RecipeStepError},
    },
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::recipes},
    diesel::prelude::*,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Recipe {
    pub id: RecipeId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub slug: String,
    pub name: String,
    pub source: String,
    pub description: String,
    pub notes: String,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStepDetail>,
}

/// Edit-form representation of a recipe with its steps and ingredients. Binds
/// the form and is the wire payload for the recipe-upsert route.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeBuilder {
    pub id: RecipeDraftId,
    pub name: String,
    pub source: String,
    pub steps: Vec<RecipeStepBuilder>,
}

/// Validation errors mirroring the builder tree, keyed by `DraftId`. Empty means
/// valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeError {
    pub name: Option<String>,
    pub steps: HashMap<RecipeStepDraftId, RecipeStepError>,
}

impl RecipeError {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }

    /// One-line digest for contexts that can't render the structured tree.
    pub fn summary(&self) -> String {
        let mut msgs = Vec::new();

        if let Some(m) = &self.name {
            msgs.push(format!("name: {m}"));
        }
        for step in self.steps.values() {
            if let Some(m) = &step.instruction {
                msgs.push(format!("a step: {m}"));
            }
            if let Some(m) = &step.duration {
                msgs.push(format!("a step timer: {m}"));
            }
            for ing in step.ingredients.values() {
                if let Some(m) = &ing.quantity {
                    msgs.push(format!("an ingredient: {m}"));
                }
            }
        }

        if msgs.is_empty() {
            "invalid recipe".to_string()
        } else {
            msgs.join("; ")
        }
    }
}

impl From<RecipeDetail> for RecipeBuilder {
    fn from(detail: RecipeDetail) -> Self {
        Self {
            id: detail.recipe.id.into(),
            name: detail.recipe.name,
            source: detail.recipe.source,
            steps: detail.steps.into_iter().map(Into::into).collect(),
        }
    }
}

impl RecipeBuilder {
    pub fn new() -> Self {
        Self {
            steps: vec![RecipeStepBuilder::default()],
            ..Default::default()
        }
    }

    /// Validates own fields, then recurses into each step.
    pub fn validate(&self) -> Result<(), RecipeError> {
        let mut err = RecipeError::default();

        if Name::parse(&self.name).is_err() {
            err.name = Some("name is required".to_string());
        }

        for step in &self.steps {
            if let Err(e) = step.validate() {
                err.steps.insert(step.id, e);
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}
