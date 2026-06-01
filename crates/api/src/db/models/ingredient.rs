use {
    crate::{
        grocery_section::GrocerySection,
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
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Ingredient {
    pub id: IngredientId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<GrocerySection>,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = ingredients))]
pub struct NewIngredient<'a> {
    pub book_id: BookId,
    pub name: &'a str,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<GrocerySection>,
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
