//! Unified shopping-list API used by the UI. Dispatches to one of two
//! backends, mirroring the meal pattern in [`crate::meals`]:
//!
//! - Positive ids and authenticated `create`/`list` calls go to the server
//!   functions in [`crate::remote_shopping`].
//! - Negative ids and unauthenticated `create`/`list` calls go to
//!   [`web_client::shopping_lists`], which talks to browser `localStorage`.

use types::{
    NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail, aggregate_from_meal,
};

use crate::remote_shopping;

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

pub async fn get_shopping_list(id: i64) -> Result<ShoppingListDetail, String> {
    if id < 0 {
        get_local(id)
    } else {
        remote_shopping::get_shopping_list(id).await.map_err(err)
    }
}

pub async fn create_shopping_list(
    input: NewShoppingList,
    authenticated: bool,
) -> Result<i64, String> {
    if authenticated {
        remote_shopping::create_shopping_list(input)
            .await
            .map_err(err)
    } else {
        create_local(input)
    }
}

/// Build a shopping list from an existing meal by aggregating its
/// ingredients. The created list's storage backend is chosen the same way as
/// for an empty list — authenticated → server, otherwise local.
pub async fn create_from_meal(meal_key: String, authenticated: bool) -> Result<i64, String> {
    let meal = crate::meals::get_meal(meal_key).await?;
    let sections = remote_shopping::list_ingredient_sections()
        .await
        .map_err(err)
        .unwrap_or_default();
    let items = aggregate_from_meal(&meal, &sections);
    let input = NewShoppingList {
        name: meal.meal.name.clone(),
        items,
    };
    create_shopping_list(input, authenticated).await
}

pub async fn delete_shopping_list(id: i64) -> Result<(), String> {
    if id < 0 {
        delete_local(id)
    } else {
        remote_shopping::delete_shopping_list(id).await.map_err(err)
    }
}

pub async fn add_item(list_id: i64, item: NewShoppingListItem) -> Result<i64, String> {
    if list_id < 0 {
        add_item_local(list_id, item)
    } else {
        remote_shopping::add_shopping_list_item(list_id, item)
            .await
            .map_err(err)
    }
}

pub async fn set_item_checked(item_id: i64, checked: bool) -> Result<(), String> {
    if item_id < 0 {
        set_item_checked_local(item_id, checked)
    } else {
        remote_shopping::set_shopping_list_item_checked(item_id, checked)
            .await
            .map_err(err)
    }
}

pub async fn delete_item(item_id: i64) -> Result<(), String> {
    if item_id < 0 {
        delete_item_local(item_id)
    } else {
        remote_shopping::delete_shopping_list_item(item_id)
            .await
            .map_err(err)
    }
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

fn get_local(id: i64) -> Result<ShoppingListDetail, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::get_shopping_list(id).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = id;
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn create_local(input: NewShoppingList) -> Result<i64, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::create_shopping_list(input).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = input;
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn delete_local(id: i64) -> Result<(), String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::delete_shopping_list(id).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = id;
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn add_item_local(list_id: i64, item: NewShoppingListItem) -> Result<i64, String> {
    #[cfg(feature = "web")]
    {
        web_client::shopping_lists::add_item(list_id, item).map_err(err)
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (list_id, item);
        Err("local shopping-list storage unavailable on this target".into())
    }
}

fn set_item_checked_local(item_id: i64, checked: bool) -> Result<(), String> {
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

fn delete_item_local(item_id: i64) -> Result<(), String> {
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
