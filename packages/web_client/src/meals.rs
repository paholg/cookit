//! Browser-localStorage backend for meals owned by unauthenticated users.
//!
//! Storage layout:
//! - `cookit:meals:index`   — `Vec<i64>` of every locally stored meal id.
//! - `cookit:meals:next_id` — `i64`, starts at -1 and decrements on each create
//!   so local ids stay negative and never collide with SQLite-issued positives.
//! - `cookit:meal:<id>`     — `StoredMeal` JSON blob.

use anyhow::{Context, Result, anyhow};
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use types::{Meal, NewMeal, NewMealRecipe};

const INDEX_KEY: &str = "cookit:meals:index";
const NEXT_ID_KEY: &str = "cookit:meals:next_id";

fn meal_key(id: i64) -> String {
    format!("cookit:meal:{id}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMeal {
    pub name: String,
    pub recipes: Vec<NewMealRecipe>,
}

fn read_index() -> Result<Vec<i64>> {
    match LocalStorage::get::<Vec<i64>>(INDEX_KEY) {
        Ok(v) => Ok(v),
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(anyhow!("read local meal index: {e}")),
    }
}

fn write_index(index: &[i64]) -> Result<()> {
    LocalStorage::set(INDEX_KEY, index).map_err(|e| anyhow!("write local meal index: {e}"))
}

fn alloc_id() -> Result<i64> {
    let next = match LocalStorage::get::<i64>(NEXT_ID_KEY) {
        Ok(n) => n,
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => -1,
        Err(e) => return Err(anyhow!("read local meal next_id: {e}")),
    };

    let after = next
        .checked_sub(1)
        .ok_or_else(|| anyhow!("local meal id space exhausted"))?;
    LocalStorage::set(NEXT_ID_KEY, after).map_err(|e| anyhow!("write local meal next_id: {e}"))?;

    Ok(next)
}

pub fn list_meals() -> Result<Vec<Meal>> {
    let index = read_index()?;
    let mut out = Vec::with_capacity(index.len());

    for id in index {
        let stored: StoredMeal =
            LocalStorage::get(meal_key(id)).with_context(|| format!("read local meal {id}"))?;
        out.push(Meal {
            id,
            user_id: None,
            name: stored.name,
        });
    }

    Ok(out)
}

pub fn get_stored(id: i64) -> Result<StoredMeal> {
    LocalStorage::get(meal_key(id)).with_context(|| format!("read local meal {id}"))
}

pub fn create_meal(input: NewMeal) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let id = alloc_id()?;
    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };

    LocalStorage::set(meal_key(id), &stored).with_context(|| format!("write local meal {id}"))?;

    let mut index = read_index()?;
    index.push(id);
    write_index(&index)?;

    Ok(id)
}

pub fn update_meal(id: i64, input: NewMeal) -> Result<()> {
    if id >= 0 {
        return Err(anyhow!("local update called with non-local meal id {id}"));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_recipes(&input.recipes)?;

    let index = read_index()?;
    if !index.contains(&id) {
        return Err(anyhow!("local meal {id} not found"));
    }

    let stored = StoredMeal {
        name: name.to_string(),
        recipes: input.recipes,
    };
    LocalStorage::set(meal_key(id), &stored).with_context(|| format!("write local meal {id}"))?;

    Ok(())
}

pub fn delete_meal(id: i64) -> Result<()> {
    if id >= 0 {
        return Err(anyhow!("local delete called with non-local meal id {id}"));
    }
    let mut index = read_index()?;
    let before = index.len();
    index.retain(|&x| x != id);
    if index.len() == before {
        return Err(anyhow!("local meal {id} not found"));
    }
    write_index(&index)?;
    LocalStorage::delete(meal_key(id));
    Ok(())
}

fn validate_recipes(recipes: &[NewMealRecipe]) -> Result<()> {
    for (idx, mr) in recipes.iter().enumerate() {
        if !mr.multiplier.is_finite() || mr.multiplier <= 0.0 {
            return Err(anyhow!(
                "recipe {} multiplier must be a positive number, got {}",
                idx + 1,
                mr.multiplier
            ));
        }
    }
    Ok(())
}
