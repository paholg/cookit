//! Browser-localStorage backend for shopping lists owned by unauthenticated
//! users (and any local list created while logged in).
//!
//! Storage layout (mirrors [`crate::meals`]):
//! - `cookit:shopping:index`    — `Vec<i64>` of every locally stored list id.
//! - `cookit:shopping:next_id`  — next list id, starts at -1, decrements.
//! - `cookit:shopping:items_next_id` — next item id, starts at -1, decrements.
//! - `cookit:shopping:<id>`     — `StoredList` JSON blob.

use anyhow::{Context, Result, anyhow};
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use types::{
    GrocerySection, NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail,
    ShoppingListItem, Unit,
};

const INDEX_KEY: &str = "cookit:shopping:index";
const NEXT_LIST_ID_KEY: &str = "cookit:shopping:next_id";
const NEXT_ITEM_ID_KEY: &str = "cookit:shopping:items_next_id";

fn list_key(id: i64) -> String {
    format!("cookit:shopping:{id}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredList {
    name: String,
    items: Vec<StoredItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredItem {
    id: i64,
    name: String,
    grocery_section: Option<GrocerySection>,
    quantity: Option<f64>,
    unit: Option<Unit>,
    checked: bool,
    position: i64,
}

fn read_index() -> Result<Vec<i64>> {
    match LocalStorage::get::<Vec<i64>>(INDEX_KEY) {
        Ok(v) => Ok(v),
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(anyhow!("read local shopping index: {e}")),
    }
}

fn write_index(index: &[i64]) -> Result<()> {
    LocalStorage::set(INDEX_KEY, index).map_err(|e| anyhow!("write local shopping index: {e}"))
}

fn alloc_id(key: &str) -> Result<i64> {
    let next = match LocalStorage::get::<i64>(key) {
        Ok(n) => n,
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => -1,
        Err(e) => return Err(anyhow!("read {key}: {e}")),
    };
    let after = next
        .checked_sub(1)
        .ok_or_else(|| anyhow!("local id space exhausted for {key}"))?;
    LocalStorage::set(key, after).map_err(|e| anyhow!("write {key}: {e}"))?;
    Ok(next)
}

fn read_stored(id: i64) -> Result<StoredList> {
    LocalStorage::get(list_key(id)).with_context(|| format!("read local shopping list {id}"))
}

fn write_stored(id: i64, stored: &StoredList) -> Result<()> {
    LocalStorage::set(list_key(id), stored)
        .with_context(|| format!("write local shopping list {id}"))
}

pub fn list_shopping_lists() -> Result<Vec<ShoppingList>> {
    let index = read_index()?;
    let mut out = Vec::with_capacity(index.len());
    for id in index {
        let stored = read_stored(id)?;
        out.push(ShoppingList {
            id,
            user_id: None,
            name: stored.name,
        });
    }
    Ok(out)
}

pub fn get_shopping_list(id: i64) -> Result<ShoppingListDetail> {
    let stored = read_stored(id)?;
    let items = stored
        .items
        .into_iter()
        .map(|it| ShoppingListItem {
            id: it.id,
            name: it.name,
            grocery_section: it.grocery_section,
            quantity: it.quantity,
            unit: it.unit,
            checked: it.checked,
            position: it.position,
        })
        .collect();
    Ok(ShoppingListDetail {
        list: ShoppingList {
            id,
            user_id: None,
            name: stored.name,
        },
        items,
    })
}

pub fn create_shopping_list(input: NewShoppingList) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("shopping list name is required"));
    }

    let list_id = alloc_id(NEXT_LIST_ID_KEY)?;
    let mut items = Vec::with_capacity(input.items.len());
    for (idx, ni) in input.items.into_iter().enumerate() {
        let item_id = alloc_id(NEXT_ITEM_ID_KEY)?;
        items.push(StoredItem {
            id: item_id,
            name: ni.name.trim().to_string(),
            grocery_section: ni.grocery_section,
            quantity: ni.quantity,
            unit: ni.unit,
            checked: false,
            position: idx as i64,
        });
    }

    write_stored(
        list_id,
        &StoredList {
            name: name.to_string(),
            items,
        },
    )?;

    let mut index = read_index()?;
    index.push(list_id);
    write_index(&index)?;
    Ok(list_id)
}

pub fn delete_shopping_list(id: i64) -> Result<()> {
    if id >= 0 {
        return Err(anyhow!("local delete called with non-local list id {id}"));
    }
    let mut index = read_index()?;
    let before = index.len();
    index.retain(|&x| x != id);
    if index.len() == before {
        return Err(anyhow!("local shopping list {id} not found"));
    }
    write_index(&index)?;
    LocalStorage::delete(list_key(id));
    Ok(())
}

pub fn add_item(list_id: i64, item: NewShoppingListItem) -> Result<i64> {
    if item.name.trim().is_empty() {
        return Err(anyhow!("item name is required"));
    }
    let mut stored = read_stored(list_id)?;
    let item_id = alloc_id(NEXT_ITEM_ID_KEY)?;
    let position = stored
        .items
        .iter()
        .map(|i| i.position)
        .max()
        .map(|p| p + 1)
        .unwrap_or(0);
    stored.items.push(StoredItem {
        id: item_id,
        name: item.name.trim().to_string(),
        grocery_section: item.grocery_section,
        quantity: item.quantity,
        unit: item.unit,
        checked: false,
        position,
    });
    write_stored(list_id, &stored)?;
    Ok(item_id)
}

pub fn set_item_checked(item_id: i64, checked: bool) -> Result<()> {
    mutate_item(item_id, |it| {
        it.checked = checked;
    })
}

pub fn delete_item(item_id: i64) -> Result<()> {
    let index = read_index()?;
    for list_id in index {
        let mut stored = read_stored(list_id)?;
        let before = stored.items.len();
        stored.items.retain(|i| i.id != item_id);
        if stored.items.len() != before {
            write_stored(list_id, &stored)?;
            return Ok(());
        }
    }
    Err(anyhow!("local shopping list item {item_id} not found"))
}

fn mutate_item(item_id: i64, f: impl FnOnce(&mut StoredItem)) -> Result<()> {
    let index = read_index()?;
    for list_id in index {
        let mut stored = read_stored(list_id)?;
        if let Some(it) = stored.items.iter_mut().find(|i| i.id == item_id) {
            f(it);
            write_stored(list_id, &stored)?;
            return Ok(());
        }
    }
    Err(anyhow!("local shopping list item {item_id} not found"))
}
