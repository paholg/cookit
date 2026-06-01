//! Browser-localStorage backend for meals owned by unauthenticated users.
//!
//! Storage layout:
//! - `cookit:meals:index` — `Vec<String>` of every locally stored meal slug.
//! - `cookit:meal:<slug>` — `StoredMeal` JSON blob.
//!
//! Local meal slugs always start with `local-` so the dispatch in
//! `api::meals` can route them to this backend without ambiguity.

use {
    anyhow::{Context, Result, anyhow},
    gloo_storage::{LocalStorage, Storage},
    serde::{Deserialize, Serialize},
    uuid::Uuid,
};

const INDEX_KEY: &str = "cookit:meals:index";
const LOCAL_PREFIX: &str = "local-";

fn meal_storage_key(slug: &str) -> String {
    format!("cookit:meal:{slug}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMeal {
    pub name: String,
    pub recipes: Vec<NewMealRecipe>,
}

fn read_index() -> Result<Vec<String>> {
    match LocalStorage::get::<Vec<String>>(INDEX_KEY) {
        Ok(v) => Ok(v),
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(anyhow!("read local meal index: {e}")),
    }
}

fn write_index(index: &[String]) -> Result<()> {
    LocalStorage::set(INDEX_KEY, index).map_err(|e| anyhow!("write local meal index: {e}"))
}

fn alloc_slug(name: &str, index: &[String]) -> String {
    let base = format!("{LOCAL_PREFIX}{}", slugify(name));
    let mut candidate = base.clone();
    let mut n: u32 = 2;
    while index.iter().any(|k| k == &candidate) {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    candidate
}

/// A nil UUID is used as a placeholder ID for locally-stored meals. The slug
/// is the actual stable local identifier; the ID is only needed to satisfy the
/// `types::Meal` shape.
fn local_meal_id() -> MealId {
    MealId::from_uuid(Uuid::nil())
}

pub fn list_meals() -> Result<Vec<Meal>> {
    let index = read_index()?;
    let mut out = Vec::with_capacity(index.len());
    for slug in index {
        let stored: StoredMeal = LocalStorage::get(meal_storage_key(&slug))
            .with_context(|| format!("read local meal `{slug}`"))?;
        out.push(Meal {
            id: local_meal_id(),
            slug,
            name: stored.name,
        });
    }
    Ok(out)
}

pub fn get_stored(slug: &str) -> Result<StoredMeal> {
    LocalStorage::get(meal_storage_key(slug)).with_context(|| format!("read local meal `{slug}`"))
}

pub fn create_meal(input: NewMeal) -> Result<String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let mut index = read_index()?;
    let slug = alloc_slug(name, &index);

    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };
    LocalStorage::set(meal_storage_key(&slug), &stored)
        .with_context(|| format!("write local meal `{slug}`"))?;

    index.push(slug.clone());
    write_index(&index)?;

    Ok(slug)
}

pub fn update_meal(slug: &str, input: NewMeal) -> Result<()> {
    if !slug.starts_with(LOCAL_PREFIX) {
        return Err(anyhow!(
            "local update called with non-local meal slug `{slug}`"
        ));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let index = read_index()?;
    if !index.iter().any(|k| k == slug) {
        return Err(anyhow!("local meal `{slug}` not found"));
    }

    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };
    LocalStorage::set(meal_storage_key(slug), &stored)
        .with_context(|| format!("write local meal `{slug}`"))?;

    Ok(())
}

pub fn delete_meal(slug: &str) -> Result<()> {
    if !slug.starts_with(LOCAL_PREFIX) {
        return Err(anyhow!(
            "local delete called with non-local meal slug `{slug}`"
        ));
    }
    let mut index = read_index()?;
    let before = index.len();
    index.retain(|k| k != slug);
    if index.len() == before {
        return Err(anyhow!("local meal `{slug}` not found"));
    }
    write_index(&index)?;
    LocalStorage::delete(meal_storage_key(slug));
    Ok(())
}

fn validate_recipes(recipes: &[NewMealRecipe]) -> Result<()> {
    let mut seen = std::collections::HashSet::with_capacity(recipes.len());
    for (idx, mr) in recipes.iter().enumerate() {
        if !mr.multiplier.is_finite() || mr.multiplier <= 0.0 {
            return Err(anyhow!(
                "recipe {} multiplier must be a positive number, got {}",
                idx + 1,
                mr.multiplier
            ));
        }
        if mr.recipe_slug.is_empty() {
            return Err(anyhow!("recipe {} is missing a slug", idx + 1));
        }
        if !seen.insert(mr.recipe_slug.clone()) {
            return Err(anyhow!(
                "recipe {} (`{}`) appears more than once",
                idx + 1,
                mr.recipe_slug,
            ));
        }
    }
    Ok(())
}
