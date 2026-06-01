//! Unified meal API used by the UI. Dispatches every call to one of two
//! backends:
//!
//! - Slugs prefixed with `local-` and unauthenticated `create`/`list` calls
//!   go to [`web_client::meals`], which talks to browser `localStorage`.
//! - All other slugs (and authenticated `create`/`list` calls) go to the
//!   server functions in [`crate::remote`], which talk to the PostgreSQL DB.

use {
    crate::remote,
    types::{
        Meal, MealDetail, MealRecipe, NewMeal,
        id::{MealId, MealRecipeId},
    },
    uuid::Uuid,
};

const LOCAL_PREFIX: &str = "local-";

fn is_local(slug: &str) -> bool {
    slug.starts_with(LOCAL_PREFIX)
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

pub async fn get_meal(slug: String) -> Result<MealDetail, String> {
    if is_local(&slug) {
        get_local_meal(slug).await
    } else {
        remote::get_meal(slug).await.map_err(err)
    }
}

pub async fn create_meal(input: NewMeal, authenticated: bool) -> Result<String, String> {
    if authenticated {
        remote::create_meal(input).await.map_err(err)
    } else {
        create_local(input)
    }
}

pub async fn update_meal(slug: String, input: NewMeal) -> Result<(), String> {
    if is_local(&slug) {
        update_local(slug, input)
    } else {
        remote::update_meal(slug, input).await.map_err(err)
    }
}

pub async fn delete_meal(slug: String) -> Result<(), String> {
    if is_local(&slug) {
        delete_local(slug)
    } else {
        remote::delete_meal(slug).await.map_err(err)
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

fn update_local(slug: String, input: NewMeal) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::update_meal(&slug, input).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (slug, input);
        Err("local meal storage unavailable on this target".into())
    }
}

fn delete_local(slug: String) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::meals::delete_meal(&slug).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = slug;
        Err("local meal storage unavailable on this target".into())
    }
}

/// Compose a full [`MealDetail`] for a locally-stored meal by fetching each
/// referenced recipe from the server function. Recipes are not stored locally
/// — only the multiplier and the recipe slug.
async fn get_local_meal(slug: String) -> Result<MealDetail, String> {
    let stored = read_local_stored(&slug)?;

    let mut recipes = Vec::with_capacity(stored.recipes.len());
    for (position, mr) in stored.recipes.into_iter().enumerate() {
        let recipe = crate::get_recipe(mr.recipe_slug).await.map_err(err)?;
        recipes.push(MealRecipe {
            id: MealRecipeId::from_uuid(Uuid::nil()),
            multiplier: mr.multiplier,
            position: position as i32,
            recipe_detail: recipe,
        });
    }

    Ok(MealDetail {
        id: MealId::from_uuid(Uuid::nil()),
        slug,
        name: stored.name,
        recipes,
    })
}

#[cfg(feature = "web")]
fn read_local_stored(slug: &str) -> Result<web_client::meals::StoredMeal, String> {
    web_client::meals::get_stored(slug).map_err(err)
}

#[cfg(not(feature = "web"))]
fn read_local_stored(slug: &str) -> Result<LocalStub, String> {
    let _ = slug;
    Err("local meal storage unavailable on this target".into())
}

#[cfg(not(feature = "web"))]
struct LocalStub {
    name: String,
    recipes: Vec<types::NewMealRecipe>,
}
