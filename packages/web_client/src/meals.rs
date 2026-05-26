//! Browser-localStorage backend for meals owned by unauthenticated users.
//!
//! Storage layout:
//! - `cookit:meals:index` — `Vec<String>` of every locally stored meal key.
//! - `cookit:meal:<key>`  — `StoredMeal` JSON blob.
//!
//! Local meal keys always start with `local-` so the dispatch in
//! `api::meals` can route them to this backend without ambiguity. The server
//! strips any leading `local-` from server-generated meal keys so the two
//! namespaces never collide.

use anyhow::{Context, Result, anyhow};
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use types::{Meal, NewMeal, NewMealRecipe, slugify};

const INDEX_KEY: &str = "cookit:meals:index";
const LOCAL_PREFIX: &str = "local-";

fn meal_storage_key(key: &str) -> String {
    format!("cookit:meal:{key}")
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

/// Allocate a fresh `local-`-prefixed key derived from `name`, suffixing
/// with `-2`, `-3`, … if needed to avoid collisions with the current local
/// index.
fn alloc_key(name: &str, index: &[String]) -> String {
    let base = format!("{LOCAL_PREFIX}{}", slugify(name));
    let mut candidate = base.clone();
    let mut n: u32 = 2;
    while index.iter().any(|k| k == &candidate) {
        candidate = format!("{base}-{n}");
        n += 1;
    }
    candidate
}

pub fn list_meals() -> Result<Vec<Meal>> {
    let index = read_index()?;
    let mut out = Vec::with_capacity(index.len());

    for key in index {
        let stored: StoredMeal = LocalStorage::get(meal_storage_key(&key))
            .with_context(|| format!("read local meal `{key}`"))?;
        out.push(Meal {
            id: 0,
            key,
            user_id: None,
            name: stored.name,
        });
    }

    Ok(out)
}

pub fn get_stored(key: &str) -> Result<StoredMeal> {
    LocalStorage::get(meal_storage_key(key)).with_context(|| format!("read local meal `{key}`"))
}

pub fn create_meal(input: NewMeal) -> Result<String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let mut index = read_index()?;
    let key = alloc_key(name, &index);

    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };
    LocalStorage::set(meal_storage_key(&key), &stored)
        .with_context(|| format!("write local meal `{key}`"))?;

    index.push(key.clone());
    write_index(&index)?;

    Ok(key)
}

pub fn update_meal(key: &str, input: NewMeal) -> Result<()> {
    if !key.starts_with(LOCAL_PREFIX) {
        return Err(anyhow!(
            "local update called with non-local meal key `{key}`"
        ));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let index = read_index()?;
    if !index.iter().any(|k| k == key) {
        return Err(anyhow!("local meal `{key}` not found"));
    }

    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };
    LocalStorage::set(meal_storage_key(key), &stored)
        .with_context(|| format!("write local meal `{key}`"))?;

    Ok(())
}

pub fn delete_meal(key: &str) -> Result<()> {
    if !key.starts_with(LOCAL_PREFIX) {
        return Err(anyhow!(
            "local delete called with non-local meal key `{key}`"
        ));
    }
    let mut index = read_index()?;
    let before = index.len();
    index.retain(|k| k != key);
    if index.len() == before {
        return Err(anyhow!("local meal `{key}` not found"));
    }
    write_index(&index)?;
    LocalStorage::delete(meal_storage_key(key));
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
        if mr.recipe_key.is_empty() {
            return Err(anyhow!("recipe {} is missing a key", idx + 1));
        }
        if !seen.insert(mr.recipe_key.clone()) {
            return Err(anyhow!(
                "recipe {} (`{}`) appears more than once; each recipe can only be added to a meal once",
                idx + 1,
                mr.recipe_key,
            ));
        }
    }
    Ok(())
}
