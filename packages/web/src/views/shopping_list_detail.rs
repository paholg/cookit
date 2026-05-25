use std::collections::HashMap;

use api::shopping_lists::{add_item, delete_item, get_shopping_list, set_item_checked};
use dioxus::prelude::*;
use strum::IntoEnumIterator;
use types::{GrocerySection, NewShoppingListItem, ShoppingListItem, Unit, UnitKind};
use ui::icons::TrashIcon;

use super::format::format_quantity;
use crate::views::WakeLockToggle;

#[component]
pub fn ShoppingListDetail(id: i64) -> Element {
    let mut list = use_resource(move || get_shopping_list(id));

    let title = list
        .cloned()
        .and_then(|r| r.ok())
        .map(|d| d.list.name)
        .unwrap_or_else(|| "Shopping list".to_string());

    let body = match list.cloned() {
        Some(Ok(detail)) => {
            let total = detail.items.len();
            let checked = detail.items.iter().filter(|i| i.checked).count();
            let groups = group_by_section(&detail.items);

            rsx! {
                article { class: "shopping-list",
                    header { class: "shopping-header",
                        h1 { "{detail.list.name}" }
                        span { class: "shopping-count",
                            "{checked} / {total}"
                        }
                        WakeLockToggle {}
                    }

                    if total == 0 {
                        p { class: "empty", "This list is empty. Add an item below." }
                    }

                    for (section, items) in groups {
                        SectionBlock {
                            key: "{section_key(&section)}",
                            section,
                            items,
                            on_change: move |_| list.restart(),
                        }
                    }

                    AddItemForm {
                        list_id: id,
                        on_added: move |_| list.restart(),
                    }
                }
            }
        }
        Some(Err(e)) => rsx! { p { class: "error", "Error loading shopping list: {e}" } },
        None => rsx! { p { "Loading..." } },
    };

    rsx! {
        document::Title { "{title}" }
        {body}
    }
}

/// Group items by `GrocerySection` in enum-declaration order, with a trailing
/// `None` bucket for items without a section.
fn group_by_section(
    items: &[ShoppingListItem],
) -> Vec<(Option<GrocerySection>, Vec<ShoppingListItem>)> {
    let mut buckets: HashMap<Option<GrocerySection>, Vec<ShoppingListItem>> = HashMap::new();
    for it in items {
        buckets
            .entry(it.grocery_section)
            .or_default()
            .push(it.clone());
    }
    let mut out: Vec<(Option<GrocerySection>, Vec<ShoppingListItem>)> = Vec::new();
    for s in GrocerySection::iter() {
        if let Some(v) = buckets.remove(&Some(s)) {
            out.push((Some(s), v));
        }
    }
    if let Some(v) = buckets.remove(&None) {
        out.push((None, v));
    }
    out
}

fn section_key(s: &Option<GrocerySection>) -> String {
    match s {
        Some(s) => s.to_string(),
        None => "other".into(),
    }
}

fn section_label(s: &Option<GrocerySection>) -> String {
    match s {
        Some(s) => s.to_string(),
        None => "Other".into(),
    }
}

#[component]
fn SectionBlock(
    section: Option<GrocerySection>,
    items: Vec<ShoppingListItem>,
    on_change: EventHandler<()>,
) -> Element {
    // Collapse items into one row per ingredient name so multi-unit duplicates
    // (`3 lb, 2 cup flour`) share a single tap target.
    let mut by_name: Vec<(String, Vec<ShoppingListItem>)> = Vec::new();
    for it in items {
        let key = it.name.to_lowercase();
        if let Some(existing) = by_name.iter_mut().find(|(k, _)| *k == key) {
            existing.1.push(it);
        } else {
            by_name.push((key, vec![it]));
        }
    }

    // Sort unchecked rows to the top so completed items drift to the bottom of
    // each section as the shopper works.
    by_name.sort_by_key(|(_, group)| group.iter().all(|i| i.checked));

    let total = by_name.len();
    let done = by_name
        .iter()
        .filter(|(_, g)| g.iter().all(|i| i.checked))
        .count();

    rsx! {
        section { class: "shopping-section",
            header { class: "shopping-section-header",
                h2 { "{section_label(&section)}" }
                span { class: "shopping-section-count", "{done}/{total}" }
            }
            ul { class: "shopping-items",
                for (_, group) in by_name {
                    NameRow {
                        key: "{group[0].id}",
                        items: group,
                        on_change: move |_| on_change.call(()),
                    }
                }
            }
        }
    }
}

#[component]
fn NameRow(items: Vec<ShoppingListItem>, on_change: EventHandler<()>) -> Element {
    let display_name = items[0].name.clone();
    let all_checked = items.iter().all(|i| i.checked);
    let qty_text = items
        .iter()
        .map(format_qty_unit)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let item_ids: Vec<i64> = items.iter().map(|i| i.id).collect();
    let ids_for_toggle = item_ids.clone();
    let ids_for_delete = item_ids;

    let mut busy = use_signal(|| false);

    let toggle = move |_| {
        let ids = ids_for_toggle.clone();
        async move {
            if busy() {
                return;
            }
            busy.set(true);
            let target = !all_checked;
            // Sequential: name groups are 1-2 items in practice.
            for id in ids {
                let _ = set_item_checked(id, target).await;
            }
            busy.set(false);
            on_change.call(());
        }
    };

    let delete = move |e: MouseEvent| {
        e.stop_propagation();
        let ids = ids_for_delete.clone();
        async move {
            busy.set(true);
            for id in ids {
                let _ = delete_item(id).await;
            }
            busy.set(false);
            on_change.call(());
        }
    };

    let row_class = if all_checked {
        "shopping-row checked"
    } else {
        "shopping-row"
    };

    rsx! {
        li { class: row_class,
            // Whole-row tap target: the label wraps the checkbox + text so
            // taps anywhere on it toggle the group.
            label { class: "shopping-row-tap",
                input {
                    r#type: "checkbox",
                    class: "shopping-checkbox",
                    checked: all_checked,
                    disabled: busy(),
                    onchange: toggle,
                }
                span { class: "shopping-row-text",
                    if !qty_text.is_empty() {
                        span { class: "shopping-row-qty", "{qty_text}" }
                    }
                    span { class: "shopping-row-name", "{display_name}" }
                }
            }
            button {
                r#type: "button",
                class: "icon-button shopping-row-delete",
                "aria-label": "Remove",
                title: "Remove",
                disabled: busy(),
                onclick: delete,
                TrashIcon {}
            }
        }
    }
}

/// Format `quantity + unit` (handling absent qty/unit) without including the
/// ingredient name — name is rendered once per row by the caller.
fn format_qty_unit(it: &ShoppingListItem) -> String {
    let qty = it.quantity.map(format_quantity);
    let unit = it
        .unit
        .as_ref()
        .map(|u| u.label())
        .filter(|l| !l.is_empty());
    match (qty, unit) {
        (Some(q), Some(u)) => format!("{q} {u}"),
        (Some(q), None) => q,
        (None, Some(u)) => u,
        (None, None) => String::new(),
    }
}

#[component]
fn AddItemForm(list_id: i64, on_added: EventHandler<()>) -> Element {
    let mut expanded = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut qty = use_signal(String::new);
    let mut unit_text = use_signal(String::new);
    let mut unit_kind: Signal<Option<UnitKind>> = use_signal(|| None);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        let n = name.read().trim().to_string();
        if n.is_empty() {
            error.set(Some("Name is required.".into()));
            return;
        }
        let qty_val = if qty.read().trim().is_empty() {
            None
        } else {
            match qty.read().trim().parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    error.set(Some("Quantity must be a number.".into()));
                    return;
                }
            }
        };
        let unit = match (unit_kind(), unit_text.read().trim()) {
            (Some(kind), t) if !t.is_empty() => match Unit::new(kind, t) {
                Ok(u) => Some(u),
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            },
            _ => None,
        };

        submitting.set(true);
        error.set(None);
        let res = add_item(
            list_id,
            NewShoppingListItem {
                name: n,
                grocery_section: None,
                quantity: qty_val,
                unit,
            },
        )
        .await;
        submitting.set(false);
        match res {
            Ok(_) => {
                name.set(String::new());
                qty.set(String::new());
                unit_text.set(String::new());
                unit_kind.set(None);
                on_added.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };

    if !expanded() {
        return rsx! {
            div { class: "shopping-add-bar",
                button {
                    r#type: "button",
                    class: "button shopping-add-toggle",
                    onclick: move |_| expanded.set(true),
                    "+ Add item"
                }
            }
        };
    }

    rsx! {
        div { class: "shopping-add-bar",
            form {
                class: "shopping-add",
                onsubmit: submit,

                input {
                    class: "shopping-add-name",
                    placeholder: "Item",
                    value: name(),
                    autofocus: true,
                    oninput: move |e| name.set(e.value()),
                }

                div { class: "shopping-add-qty-row",
                    input {
                        placeholder: "Qty",
                        class: "shopping-add-qty",
                        inputmode: "decimal",
                        value: qty(),
                        oninput: move |e| qty.set(e.value()),
                    }
                    select {
                        class: "shopping-add-kind",
                        value: unit_kind().map(|k| k.to_string()).unwrap_or_default(),
                        onchange: move |e| {
                            use std::str::FromStr;
                            let v = e.value();
                            unit_kind.set(if v.is_empty() { None } else { UnitKind::from_str(&v).ok() });
                        },
                        option { value: "", "no unit" }
                        option { value: "mass", "mass" }
                        option { value: "volume", "volume" }
                        option { value: "count", "count" }
                        option { value: "custom", "custom" }
                    }
                    input {
                        placeholder: "Unit",
                        class: "shopping-add-unit",
                        value: unit_text(),
                        disabled: unit_kind().is_none(),
                        oninput: move |e| unit_text.set(e.value()),
                    }
                }

                if let Some(e) = error() {
                    p { class: "error inline", "{e}" }
                }

                div { class: "shopping-add-actions",
                    button {
                        r#type: "button",
                        class: "secondary",
                        onclick: move |_| {
                            expanded.set(false);
                            error.set(None);
                        },
                        "Cancel"
                    }
                    button {
                        r#type: "submit",
                        class: "button",
                        disabled: submitting(),
                        if submitting() { "Adding..." } else { "Add" }
                    }
                }
            }
        }
    }
}
