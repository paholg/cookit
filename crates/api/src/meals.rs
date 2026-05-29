//! Unified meal API used by the UI. Dispatches every call to one of two
//! backends:
//!
//! - Keys prefixed with `local-` and unauthenticated `create`/`list` calls
//!   go to [`web_client::meals`], which talks to browser `localStorage`.
//! - All other keys (and authenticated `create`/`list` calls) go to the
//!   server functions in [`crate::remote`], which talk to the SQLite DB.
//!
//! Only the lines that actually call into `web_client` are gated on the `web`
//! feature; the dispatch shape, error wrapping, and recipe composition are
//! plain Rust that rust-analyzer sees in every build.

use types::{Meal, MealDetail, MealRecipe, NewMeal};

use crate::remote;

const LOCAL_PREFIX: &str = "local-";

fn is_local(key: &str) -> bool {
    key.starts_with(LOCAL_PREFIX)
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub async fn list_meals(authenticated: bool) -> Result<Vec<Meal>, String> {
    let mut out = list_local()?;

    if authenticated {
        let server = remote::list_meals().await.map_err(err)?;
        out.extend(server);
    }

    out.sort_by_key(|m| m.name.to_lowercase());
    Ok(out)
}

pub async fn get_meal(key: String) -> Result<MealDetail, String> {
    if is_local(&key) {
        get_local_meal(key).await
    } else {
        remote::get_meal(key).await.map_err(err)
    }
}

pub async fn create_meal(input: NewMeal, authenticated: bool) -> Result<String, String> {
    if authenticated {
        remote::create_meal(input).await.map_err(err)
    } else {
        create_local(input)
    }
}

pub async fn update_meal(key: String, input: NewMeal) -> Result<(), String> {
    if is_local(&key) {
        update_local(key, input)
    } else {
        remote::update_meal(key, input).await.map_err(err)
    }
}

pub async fn delete_meal(key: String) -> Result<(), String> {
    if is_local(&key) {
        delete_local(key)
    } else {
        remote::delete_meal(key).await.map_err(err)
    }
}

// ---------- local backend bridge ----------

fn list_local() -> Result<Vec<Meal>, String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::list_meals().map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        Ok(Vec::new())
    }
}

fn create_local(input: NewMeal) -> Result<String, String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::create_meal(input).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = input;
        Err("local meal storage unavailable on this target".into())
    }
}

fn update_local(key: String, input: NewMeal) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::update_meal(&key, input).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (key, input);
        Err("local meal storage unavailable on this target".into())
    }
}

fn delete_local(key: String) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::delete_meal(&key).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = key;
        Err("local meal storage unavailable on this target".into())
    }
}

/// Compose a full [`MealDetail`] for a locally-stored meal by fetching each
/// referenced recipe from the public `get_recipe` server function. Recipes are
/// not stored locally — only the multiplier and the recipe key — so this hits
/// the network sequentially per recipe, matching the server-side traversal in
/// `server::ops::get_meal`.
async fn get_local_meal(key: String) -> Result<MealDetail, String> {
    let stored = read_local_stored(&key)?;

    let mut recipes = Vec::with_capacity(stored.recipes.len());
    for (position, mr) in stored.recipes.into_iter().enumerate() {
        let recipe = crate::get_recipe(mr.recipe_key).await.map_err(err)?;
        recipes.push(MealRecipe {
            multiplier: mr.multiplier,
            position: position as i64,
            recipe,
        });
    }

    Ok(MealDetail {
        meal: Meal {
            id: 0,
            key,
            user_id: None,
            name: stored.name,
        },
        recipes,
    })
}

#[cfg(feature = "web")]
fn read_local_stored(key: &str) -> Result<web_client::meals::StoredMeal, String> {
    web_client::meals::get_stored(key).map_err(err)
}

#[cfg(not(feature = "web"))]
fn read_local_stored(key: &str) -> Result<LocalStub, String> {
    let _ = key;
    Err("local meal storage unavailable on this target".into())
}

/// Stand-in for [`web_client::meals::StoredMeal`] when the `web` feature is
/// disabled. It is never constructed because `read_local_stored` always errs
/// in that build, but it has to satisfy the field accesses in
/// [`get_local_meal`].
#[cfg(not(feature = "web"))]
struct LocalStub {
    name: String,
    recipes: Vec<types::NewMealRecipe>,
}
