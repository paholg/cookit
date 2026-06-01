use {
    crate::{
        grocery_section::GrocerySection,
        helpers::{Name, PositiveFloat},
        id::{BookId, IngredientId},
    },
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::{
    db::{models::book::Book, prelude::*, schema::ingredients},
    session::Session,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Ingredient {
    pub id: IngredientId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
}

/// Columns for creating a bare ingredient (just a name); density and grocery
/// section are filled in later via the ingredient editor.
#[cfg(feature = "server")]
#[derive(Insertable)]
#[diesel(table_name = ingredients)]
pub(crate) struct IngredientNew<'a> {
    pub(crate) book_id: BookId,
    pub(crate) name: &'a str,
}

#[cfg(feature = "server")]
impl Ingredient {
    pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<Ingredient>> {
        let rows = ingredients::table
            .filter(ingredients::book_id.eq(session.book_id()))
            .order(ingredients::name.asc())
            .load(session.conn())
            .await?;

        Ok(rows)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(AsChangeset))]
#[cfg_attr(feature = "server", diesel(table_name = ingredients))]
pub struct IngredientUpdate {
    pub name: Name,
    pub density_g_per_ml: Option<PositiveFloat>,
    pub grocery_section: Option<GrocerySection>,
}

#[cfg(feature = "server")]
impl IngredientUpdate {
    pub async fn apply(
        self,
        id: IngredientId,
        session: &mut Session,
    ) -> anyhow::Result<Ingredient> {
        diesel::update(
            ingredients::table
                .filter(ingredients::id.eq(id))
                .filter(ingredients::book_id.eq(session.book_id())),
        )
        .set(self)
        .returning(Ingredient::as_returning())
        .get_result(session.conn())
        .await
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("ingredient {id:?} not found"))
    }
}
