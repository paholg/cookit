use {
    crate::{
        db::models::shopping_list_item::ShoppingListItemView,
        id::{BookId, ShoppingListId},
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        db::{
            conn::DbConn,
            models::{
                book::Book,
                ingredient::Ingredient,
                meal::MealDetail,
                shopping_list_item::{
                    ShoppingListItem, ShoppingListItemInput, ShoppingListItemRecord,
                },
            },
            prelude::*,
            schema::{ingredients, shopping_list_items, shopping_lists},
        },
        helpers::slugify,
        id::{IngredientId, ShoppingListItemId},
        session::Session,
    },
    anyhow::{Context, anyhow},
    std::collections::HashMap,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct ShoppingList {
    pub id: ShoppingListId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub slug: String,
    pub name: String,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[cfg(feature = "server")]
impl ShoppingList {
    pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<ShoppingList>> {
        let rows = shopping_lists::table
            .filter(shopping_lists::book_id.eq(session.book_id()))
            .order(shopping_lists::name.asc())
            .load(session.conn())
            .await?;

        Ok(rows)
    }

    /// Create an empty, named shopping list. Returns its id.
    pub async fn create(session: &mut Session, name: &str) -> anyhow::Result<ShoppingListId> {
        let book_id = session.book_id();
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "shopping list name is required");

        let slug = unique_slug(session.conn(), book_id, &slugify(name)).await?;

        let id = diesel::insert_into(shopping_lists::table)
            .values(&NewShoppingList {
                book_id,
                slug: &slug,
                name,
            })
            .returning(shopping_lists::id)
            .get_result(session.conn())
            .await
            .context("insert shopping list")?;

        Ok(id)
    }

    /// Build a new shopping list from a meal by aggregating every recipe's
    /// ingredients (scaled by each recipe's multiplier), merging rows that share
    /// an ingredient and unit. Returns the new list's id.
    pub async fn create_from_meal(
        session: &mut Session,
        meal_slug: &str,
    ) -> anyhow::Result<ShoppingListId> {
        let meal = MealDetail::get(session, meal_slug).await?;
        let aggregated = aggregate_meal(&meal);

        let id = ShoppingList::create(session, &meal.meal.name).await?;
        let book_id = session.book_id();

        for (position, agg) in aggregated.into_iter().enumerate() {
            diesel::insert_into(shopping_list_items::table)
                .values(&ShoppingListItemRecord {
                    book_id,
                    shopping_list_id: id,
                    position: position as i32,
                    quantity: agg.quantity,
                    unit_kind: agg.unit_kind,
                    unit: agg.unit,
                    ingredient_id: Some(agg.ingredient_id),
                    text: None,
                })
                .execute(session.conn())
                .await
                .context("insert aggregated item")?;
        }

        Ok(id)
    }

    /// Delete a shopping list by id within the book. Items go via FK cascade.
    pub async fn delete(session: &mut Session, id: ShoppingListId) -> anyhow::Result<()> {
        diesel::delete(
            shopping_lists::table
                .filter(shopping_lists::id.eq(id))
                .filter(shopping_lists::book_id.eq(session.book_id())),
        )
        .execute(session.conn())
        .await
        .context("delete shopping list")?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingListDetail {
    pub list: ShoppingList,
    pub items: Vec<ShoppingListItemView>,
}

#[cfg(feature = "server")]
impl ShoppingListDetail {
    pub async fn get(session: &mut Session, id: ShoppingListId) -> anyhow::Result<Self> {
        let list: ShoppingList = shopping_lists::table
            .filter(shopping_lists::id.eq(id))
            .filter(shopping_lists::book_id.eq(session.book_id()))
            .first(session.conn())
            .await?;

        let rows: Vec<ShoppingListItem> = ShoppingListItem::belonging_to(&list)
            .order(shopping_list_items::position.asc())
            .load(session.conn())
            .await?;

        // Join ingredient name + section for items that reference an ingredient.
        let ingredient_ids: Vec<IngredientId> =
            rows.iter().filter_map(|r| r.ingredient_id).collect();
        let ingredients_map: HashMap<IngredientId, Ingredient> = ingredients::table
            .filter(ingredients::id.eq_any(&ingredient_ids))
            .load::<Ingredient>(session.conn())
            .await?
            .into_iter()
            .map(|i| (i.id, i))
            .collect();

        let items = rows
            .into_iter()
            .map(|row| {
                let ingredient = row.ingredient_id.and_then(|id| ingredients_map.get(&id));
                ShoppingListItemView {
                    id: row.id,
                    quantity: row.quantity,
                    unit: row.unit,
                    ingredient_name: ingredient.map(|i| i.name.as_ref().to_string()),
                    text: row.text,
                    grocery_section: ingredient.and_then(|i| i.grocery_section),
                    checked: row.checked,
                }
            })
            .collect();

        Ok(ShoppingListDetail { list, items })
    }
}

/// Append a manually entered item to a list. Returns the new item's id.
#[cfg(feature = "server")]
pub async fn add_item(
    session: &mut Session,
    list_id: ShoppingListId,
    input: ShoppingListItemInput,
) -> anyhow::Result<ShoppingListItemId> {
    let book_id = session.book_id();

    // Confirm the list is in this book before adding to it.
    let position: i64 = ShoppingListItem::belonging_to(&list_owned(session, list_id).await?)
        .count()
        .get_result(session.conn())
        .await
        .context("count items")?;

    let record = input.record(book_id, list_id, position as i32)?;

    let id = diesel::insert_into(shopping_list_items::table)
        .values(&record)
        .returning(shopping_list_items::id)
        .get_result(session.conn())
        .await
        .context("insert shopping list item")?;

    Ok(id)
}

/// Check or uncheck a single item.
#[cfg(feature = "server")]
pub async fn set_item_checked(
    session: &mut Session,
    item_id: ShoppingListItemId,
    checked: bool,
) -> anyhow::Result<()> {
    let affected = diesel::update(
        shopping_list_items::table
            .filter(shopping_list_items::id.eq(item_id))
            .filter(shopping_list_items::book_id.eq(session.book_id())),
    )
    .set(shopping_list_items::checked.eq(checked))
    .execute(session.conn())
    .await
    .context("update item checked")?;

    anyhow::ensure!(affected == 1, "item {item_id:?} not found");
    Ok(())
}

/// Remove a single item.
#[cfg(feature = "server")]
pub async fn delete_item(session: &mut Session, item_id: ShoppingListItemId) -> anyhow::Result<()> {
    diesel::delete(
        shopping_list_items::table
            .filter(shopping_list_items::id.eq(item_id))
            .filter(shopping_list_items::book_id.eq(session.book_id())),
    )
    .execute(session.conn())
    .await
    .context("delete shopping list item")?;

    Ok(())
}

/// Fetch a list scoped to the session's book, erroring if it isn't there.
#[cfg(feature = "server")]
async fn list_owned(
    session: &mut Session,
    list_id: ShoppingListId,
) -> anyhow::Result<ShoppingList> {
    shopping_lists::table
        .filter(shopping_lists::id.eq(list_id))
        .filter(shopping_lists::book_id.eq(session.book_id()))
        .first(session.conn())
        .await
        .optional()
        .context("look up shopping list")?
        .ok_or_else(|| anyhow!("shopping list {list_id:?} not found"))
}

/// One aggregated row: an ingredient and a unit, with quantities summed.
#[cfg(feature = "server")]
struct Aggregated {
    ingredient_id: IngredientId,
    quantity: Option<f64>,
    unit_kind: Option<String>,
    unit: Option<String>,
}

/// Aggregate a meal's ingredients into shopping rows. Rows with the same
/// `(ingredient_id, unit)` are merged by summing quantities; different units
/// stay separate. Insertion order is preserved.
#[cfg(feature = "server")]
fn aggregate_meal(meal: &MealDetail) -> Vec<Aggregated> {
    let mut by_key: HashMap<(IngredientId, Option<String>), usize> = HashMap::new();
    let mut out: Vec<Aggregated> = Vec::new();

    for mr in &meal.recipes {
        let multiplier = mr.meal_recipe.multiplier;
        for step in &mr.recipe.steps {
            for ing in &step.ingredients {
                let ingredient_id = ing.rsi.ingredient_id;
                let scaled = ing.rsi.quantity.map(|q| q * multiplier);
                let key = (ingredient_id, ing.rsi.unit.clone());

                if let Some(&idx) = by_key.get(&key) {
                    let existing = &mut out[idx];
                    existing.quantity = match (existing.quantity, scaled) {
                        (Some(a), Some(b)) => Some(a + b),
                        (Some(a), None) => Some(a),
                        (None, b) => b,
                    };
                } else {
                    out.push(Aggregated {
                        ingredient_id,
                        quantity: scaled,
                        unit_kind: ing.rsi.unit_kind.clone(),
                        unit: ing.rsi.unit.clone(),
                    });
                    by_key.insert(key, out.len() - 1);
                }
            }
        }
    }

    out
}

#[cfg(feature = "server")]
#[derive(Insertable)]
#[diesel(table_name = shopping_lists)]
struct NewShoppingList<'a> {
    book_id: BookId,
    slug: &'a str,
    name: &'a str,
}

/// `base`, or `base-2`, `base-3`, … until unused within the book.
#[cfg(feature = "server")]
async fn unique_slug(conn: &mut DbConn, book_id: BookId, base: &str) -> anyhow::Result<String> {
    let mut candidate = base.to_string();
    let mut n: u32 = 2;

    loop {
        let taken: bool = diesel::select(diesel::dsl::exists(
            shopping_lists::table
                .filter(shopping_lists::book_id.eq(book_id))
                .filter(shopping_lists::slug.eq(candidate.as_str())),
        ))
        .get_result(conn)
        .await
        .context("probe shopping list slug")?;

        if !taken {
            return Ok(candidate);
        }

        candidate = format!("{base}-{n}");
        n = n
            .checked_add(1)
            .ok_or_else(|| anyhow!("slug space exhausted"))?;
    }
}
