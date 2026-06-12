use {
    crate::{
        Name, Slug, Timestamp,
        id::{BookId, MealDraftId, MealId, MealRecipeDraftId},
        models::meal_recipe::{MealRecipeBuilder, MealRecipeDetail, MealRecipeError},
    },
    db_macros::DieselRpc,
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};
#[cfg(feature = "server")]
use {
    crate::{models::book::Book, schema::meals},
    diesel::prelude::{Associations, HasQuery, Identifiable},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DieselRpc)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[diesel_rpc(table = meals)]
pub struct Meal {
    #[diesel_rpc(create, read, update, delete)]
    pub id: MealId,
    #[diesel_rpc(create)]
    pub book_id: BookId,
    #[diesel_rpc(read)]
    pub updated_at: Timestamp,
    #[diesel_rpc(create, read)]
    pub slug: Slug,
    #[diesel_rpc(create, read, update)]
    pub name: Name,
    #[diesel_rpc(read)]
    pub deleted_at: Option<Timestamp>,
    #[diesel_rpc(read)]
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealDetail {
    pub meal: Meal,
    pub recipes: Vec<MealRecipeDetail>,
}

/// Edit-form representation of a meal with its recipe rows. Binds the form and
/// is the wire payload for the meal-upsert route.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealBuilder {
    pub id: MealDraftId,
    pub name: String,
    pub recipes: Vec<MealRecipeBuilder>,
}

/// Validation errors mirroring the builder, keyed by `DraftId`. Empty means
/// valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealError {
    pub name: Option<String>,
    pub recipes: HashMap<MealRecipeDraftId, MealRecipeError>,
}

impl MealError {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.recipes.is_empty()
    }

    /// One-line digest for contexts that can't render the structured tree.
    pub fn summary(&self) -> String {
        let mut msgs = Vec::new();

        if let Some(m) = &self.name {
            msgs.push(format!("name: {m}"));
        }
        for row in self.recipes.values() {
            if let Some(m) = &row.multiplier {
                msgs.push(format!("a recipe: {m}"));
            }
        }

        if msgs.is_empty() {
            "invalid meal".to_string()
        } else {
            msgs.join("; ")
        }
    }
}

impl From<MealDetail> for MealBuilder {
    fn from(detail: MealDetail) -> Self {
        Self {
            id: detail.meal.id.into(),
            name: detail.meal.name.to_string(),
            recipes: detail.recipes.into_iter().map(Into::into).collect(),
        }
    }
}

impl MealBuilder {
    pub fn new() -> Self {
        Self {
            recipes: vec![MealRecipeBuilder {
                multiplier: "1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Validates own fields, then recurses into each recipe row.
    pub fn validate(&self) -> Result<(), MealError> {
        let mut err = MealError::default();

        if Name::try_new(&self.name).is_err() {
            err.name = Some("name is required".to_string());
        }

        for row in &self.recipes {
            if let Err(e) = row.validate() {
                err.recipes.insert(row.id, e);
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}
