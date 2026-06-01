//! Unified shopping-list API used by the UI. Dispatches to one of two
//! backends:
//!
//! - Slugs prefixed with `local-` and unauthenticated `create`/`list` calls
//!   go to [`web_client::shopping_lists`], which talks to browser `localStorage`.
//! - All other slugs (and authenticated calls) go to the server functions in
//!   [`crate::remote_shopping`], which talk to the PostgreSQL DB.

use {
    crate::remote_shopping,
    types::{
        NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail,
        aggregate_from_meal,
        id::{ShoppingListId, ShoppingListItemId},
    },
};

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub async fn list_shopping_lists(authenticated: bool) -> Result<Vec<ShoppingList>, String> {
    let mut out = list_local()?;
    if authenticated {
        let server = remote_shopping::list_shopping_lists().await.map_err(err)?;
        out.extend(server);
    }
    out.sort_by_key(|l| l.name.to_lowercase());
    Ok(out)
}

pub async fn get_shopping_list(id: ShoppingListId) -> Result<ShoppingListDetail, String> {
    remote_shopping::get_shopping_list(id).await.map_err(err)
}

pub async fn get_local_shopping_list(slug: String) -> Result<ShoppingListDetail, String> {
    get_local(&slug)
}

pub async fn create_shopping_list(
    input: NewShoppingList,
    authenticated: bool,
) -> Result<ShoppingListId, String> {
    if authenticated {
        remote_shopping::create_shopping_list(input)
            .await
            .map_err(err)
    } else {
        Err("creating shopping lists requires authentication".into())
    }
}

/// Build a shopping list from an existing meal by aggregating its ingredients.
pub async fn create_from_meal(
    meal_slug: String,
    authenticated: bool,
) -> Result<ShoppingListId, String> {
    let meal = crate::meals::get_meal(meal_slug).await?;
    let sections = remote_shopping::list_ingredient_sections()
        .await
        .map_err(err)
        .unwrap_or_default();
    let items = aggregate_from_meal(&meal, &sections);
    let input = NewShoppingList {
        name: meal.name.clone(),
        items,
    };
    create_shopping_list(input, authenticated).await
}

pub async fn delete_shopping_list(id: ShoppingListId) -> Result<(), String> {
    remote_shopping::delete_shopping_list(id).await.map_err(err)
}

pub async fn delete_local_shopping_list(slug: String) -> Result<(), String> {
    delete_local(&slug)
}

pub async fn add_item(
    list_id: ShoppingListId,
    item: NewShoppingListItem,
) -> Result<ShoppingListItemId, String> {
    remote_shopping::add_shopping_list_item(list_id, item)
        .await
        .map_err(err)
}

pub async fn add_item_local(
    list_slug: String,
    item: NewShoppingListItem,
) -> Result<ShoppingListItemId, String> {
    add_item_local_fn(&list_slug, item)
}

pub async fn set_item_checked(item_id: ShoppingListItemId, checked: bool) -> Result<(), String> {
    remote_shopping::set_shopping_list_item_checked(item_id, checked)
        .await
        .map_err(err)
}

pub async fn set_item_checked_local(
    item_id: ShoppingListItemId,
    checked: bool,
) -> Result<(), String> {
    set_local_item_checked(item_id, checked)
}

pub async fn delete_item(item_id: ShoppingListItemId) -> Result<(), String> {
    remote_shopping::delete_shopping_list_item(item_id)
        .await
        .map_err(err)
}

pub async fn delete_item_local(item_id: ShoppingListItemId) -> Result<(), String> {
    delete_local_item(item_id)
}

// ---------- local backend bridge ----------

fn list_local() -> Result<Vec<ShoppingList>, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::list_shopping_lists().map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        Ok(Vec::new())
    }
}

fn get_local(slug: &str) -> Result<ShoppingListDetail, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::get_shopping_list(slug).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = slug;
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn delete_local(slug: &str) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::delete_shopping_list(slug).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = slug;
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn add_item_local_fn(
    list_slug: &str,
    item: NewShoppingListItem,
) -> Result<ShoppingListItemId, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::add_item(list_slug, item).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (list_slug, item);
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn set_local_item_checked(item_id: ShoppingListItemId, checked: bool) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::set_item_checked(item_id, checked).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (item_id, checked);
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn delete_local_item(item_id: ShoppingListItemId) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::delete_item(item_id).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = item_id;
        Err("local shopping-list storage unavailable on this target".into())
    }
}
