use {
    crate::{
        db::models::recipe_step::RecipeStep,
        id::{BookId, RecipeId},
    },
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::{
    db::{models::book::Book, prelude::*, schema::recipes},
    session::Session,
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

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = recipes))]
pub(crate) struct NewRecipe<'a> {
    pub(crate) book_id: BookId,
    pub(crate) slug: &'a str,
    pub(crate) name: &'a str,
    pub(crate) source: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) notes: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStep>,
}
