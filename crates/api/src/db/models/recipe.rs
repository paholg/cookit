use {
    crate::{
        db::models::recipe_step::{RecipeStep, RecipeStepDetail},
        helpers::{Name, Slug},
        id::{BookId, RecipeId},
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        db::{
            models::{
                book::Book,
                ingredient::Ingredient,
                recipe_step_ingredient::{RecipeStepIngredient, RecipeStepIngredientDetail},
            },
            prelude::*,
            schema::{ingredients, recipe_step_ingredients, recipe_steps, recipes},
        },
        id::IngredientId,
        session::Session,
    },
    std::collections::HashMap,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub(crate) struct Recipe {
    pub(crate) id: RecipeId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) description: String,
    pub(crate) notes: String,
}

#[cfg(feature = "server")]
impl Recipe {
    // TODO: Paginate.
    pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<Recipe>> {
        let rows = recipes::table
            .filter(recipes::book_id.eq(session.book_id()))
            .order(recipes::name.asc())
            .load(session.conn())
            .await?;

        Ok(rows)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStepDetail>,
}

#[cfg(feature = "server")]
impl RecipeDetail {
    pub async fn get(session: &mut Session, slug: &str) -> anyhow::Result<Self> {
        let recipe: Recipe = recipes::table
            .filter(recipes::book_id.eq(session.book_id()))
            .filter(recipes::slug.eq(slug))
            .first(session.conn())
            .await?;

        let steps: Vec<RecipeStep> = RecipeStep::belonging_to(&recipe)
            .order(recipe_steps::position.asc())
            .load(session.conn())
            .await?;

        let rsis: Vec<RecipeStepIngredient> = RecipeStepIngredient::belonging_to(&steps)
            .order((
                recipe_step_ingredients::step_id.asc(),
                recipe_step_ingredients::position.asc(),
            ))
            .load(session.conn())
            .await?;

        let ingredient_ids: Vec<_> = rsis.iter().map(|rsi| rsi.ingredient_id).collect();

        let ingredient_map: HashMap<IngredientId, Ingredient> = ingredients::table
            .filter(ingredients::id.eq_any(&ingredient_ids))
            .load(session.conn())
            .await?
            .into_iter()
            .map(|i: Ingredient| (i.id, i))
            .collect();

        let rsis_by_step = rsis.grouped_by(&steps);

        let steps = steps
            .into_iter()
            .zip(rsis_by_step)
            .map(|(step, step_rsis)| {
                let ingredients = step_rsis
                    .into_iter()
                    .map(|rsi| {
                        let ingredient = ingredient_map[&rsi.ingredient_id].clone();
                        RecipeStepIngredientDetail { rsi, ingredient }
                    })
                    .collect();
                RecipeStepDetail { step, ingredients }
            })
            .collect();

        Ok(RecipeDetail { recipe, steps })
    }
}

#[derive(Debug)]
pub struct RecipeNew {
    pub slug: Slug,
    pub name: Name,
    pub source: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub steps: Vec<RecipeStepNew>,
}
