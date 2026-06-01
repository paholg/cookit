//! Browser-localStorage backend for shopping lists owned by unauthenticated
//! users (and any local list created while logged in).
//!
//! Storage layout:
//! - `cookit:shopping:index`   — `Vec<String>` of every locally stored list slug.
//! - `cookit:shopping:<slug>`  — `StoredList` JSON blob.
//!
//! Local list slugs always start with `local-` so the dispatch in
//! `api::shopping_lists` can route them without ambiguity.

use {
    anyhow::{Context, Result, anyhow},
    gloo_storage::{LocalStorage, Storage},
    serde::{Deserialize, Serialize},
    types::{
        GrocerySection, NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail,
        ShoppingListItem, Unit,
        id::{ShoppingListId, ShoppingListItemId},
        slugify,
    },
    uuid::Uuid,
};

const INDEX_KEY: &str = "cookit:shopping:index";
const LOCAL_PREFIX: &str = "local-";

fn list_storage_key(slug: &str) -> String {
    format!("cookit:shopping:{slug}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredList {
    name: String,
    items: Vec<StoredItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredItem {
    /// Stable UUID for this item. Generated once on creation.
    id: Uuid,
    text: Option<String>,
    grocery_section: Option<GrocerySection>,
    quantity: Option<f64>,
    unit: Option<Unit>,
    checked: bool,
    position: i32,
}

fn read_index() -> Result<Vec<String>> {
    match LocalStorage::get::<Vec<String>>(INDEX_KEY) {
        Ok(v) => Ok(v),
        Err(gloo_storage::errors::StorageError::KeyNotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(anyhow!("read local shopping index: {e}")),
    }
}

fn write_index(index: &[String]) -> Result<()> {
    LocalStorage::set(INDEX_KEY, index).map_err(|e| anyhow!("write local shopping index: {e}"))
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

fn read_stored(slug: &str) -> Result<StoredList> {
    LocalStorage::get(list_storage_key(slug))
        .with_context(|| format!("read local shopping list `{slug}`"))
}

fn write_stored(slug: &str, stored: &StoredList) -> Result<()> {
    LocalStorage::set(list_storage_key(slug), stored)
        .with_context(|| format!("write local shopping list `{slug}`"))
}

fn local_list_id() -> ShoppingListId {
    ShoppingListId::from_uuid(Uuid::nil())
}

fn stored_item_to_type(it: StoredItem) -> ShoppingListItem {
    ShoppingListItem {
        id: ShoppingListItemId::from_uuid(it.id),
        position: it.position,
        quantity: it.quantity,
        unit: it.unit,
        ingredient_id: None,
        ingredient_name: None,
        grocery_section: it.grocery_section,
        text: it.text,
        checked: it.checked,
    }
}

pub fn list_shopping_lists() -> Result<Vec<ShoppingList>> {
    let index = read_index()?;
    let mut out = Vec::with_capacity(index.len());
    for slug in index {
        let stored = read_stored(&slug)?;
        out.push(ShoppingList {
            id: local_list_id(),
            slug,
            name: stored.name,
        });
    }
    Ok(out)
}

pub fn get_shopping_list(slug: &str) -> Result<ShoppingListDetail> {
    let stored = read_stored(slug)?;
    let items = stored.items.into_iter().map(stored_item_to_type).collect();
    Ok(ShoppingListDetail {
        id: local_list_id(),
        slug: slug.to_string(),
        name: stored.name,
        items,
    })
}

pub fn create_shopping_list(input: NewShoppingList) -> Result<String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("shopping list name is required"));
    }

    let mut index = read_index()?;
    let slug = alloc_slug(name, &index);

    let items = input
        .items
        .into_iter()
        .enumerate()
        .map(|(idx, ni)| StoredItem {
            id: Uuid::new_v4(),
            text: ni.text,
            grocery_section: None,
            quantity: ni.quantity,
            unit: ni.unit,
            checked: false,
            position: idx as i32,
        })
        .collect();

    write_stored(
        &slug,
        &StoredList {
            name: name.to_string(),
            items,
        },
    )?;

    index.push(slug.clone());
    write_index(&index)?;
    Ok(slug)
}

pub fn delete_shopping_list(slug: &str) -> Result<()> {
    if !slug.starts_with(LOCAL_PREFIX) {
        return Err(anyhow!("local delete called with non-local slug `{slug}`"));
    }
    let mut index = read_index()?;
    let before = index.len();
    index.retain(|s| s != slug);
    if index.len() == before {
        return Err(anyhow!("local shopping list `{slug}` not found"));
    }
    write_index(&index)?;
    LocalStorage::delete(list_storage_key(slug));
    Ok(())
}

pub fn add_item(list_slug: &str, item: NewShoppingListItem) -> Result<ShoppingListItemId> {
    let mut stored = read_stored(list_slug)?;
    let next_pos = stored
        .items
        .iter()
        .map(|i| i.position)
        .max()
        .map(|p| p + 1)
        .unwrap_or(0);
    let id = Uuid::new_v4();
    stored.items.push(StoredItem {
        id,
        text: item.text,
        grocery_section: None,
        quantity: item.quantity,
        unit: item.unit,
        checked: false,
        position: next_pos,
    });
    write_stored(list_slug, &stored)?;
    Ok(ShoppingListItemId::from_uuid(id))
}

pub fn set_item_checked(item_id: ShoppingListItemId, checked: bool) -> Result<()> {
    mutate_item(item_id, |it| {
        it.checked = checked;
    })
}

pub fn delete_item(item_id: ShoppingListItemId) -> Result<()> {
    let index = read_index()?;
    for slug in index {
        let mut stored = read_stored(&slug)?;
        let before = stored.items.len();
        stored.items.retain(|i| i.id != *item_id.as_uuid());
        if stored.items.len() != before {
            write_stored(&slug, &stored)?;
            return Ok(());
        }
    }
    Err(anyhow!("local shopping list item not found"))
}

fn mutate_item(item_id: ShoppingListItemId, f: impl FnOnce(&mut StoredItem)) -> Result<()> {
    let index = read_index()?;
    for slug in index {
        let mut stored = read_stored(&slug)?;
        if let Some(it) = stored.items.iter_mut().find(|i| i.id == *item_id.as_uuid()) {
            f(it);
            write_stored(&slug, &stored)?;
            return Ok(());
        }
    }
    Err(anyhow!("local shopping list item not found"))
}
