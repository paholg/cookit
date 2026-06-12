use {
    crate::{WakeLockToggle, format::format_quantity, icons::TrashIcon},
    api::{
        ShoppingListItemInput, ShoppingListItemView, add_shopping_list_item,
        delete_shopping_list_item, get_shopping_list,
        grocery_section::GrocerySection,
        id::{ShoppingListId, ShoppingListItemId},
        page_title, set_shopping_list_item_checked,
    },
    dioxus::prelude::*,
    std::collections::HashMap,
    strum::IntoEnumIterator,
};

#[component]
pub fn ShoppingListDetail(id: ShoppingListId) -> Element {
    let mut list = use_server_future(move || get_shopping_list(id))?;

    let title = list
        .cloned()
        .and_then(|r| r.ok())
        .map(|d| d.list.name)
        .map(|n| page_title(&n))
        .unwrap_or_else(|| page_title("Shopping list"));

    let body = match list.cloned() {
        Some(Ok(detail)) => {
            let total = detail.items.len();
            let checked = detail.items.iter().filter(|i| i.checked).count();
            let groups = group_by_section(&detail.items);

            rsx! {
                article { class: "shopping-list",
                    header { class: "shopping-header",
                        h1 { "{detail.list.name}" }
                        span { class: "shopping-count", "{checked} / {total}" }
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

                    AddItemForm { list_id: id, on_added: move |_| list.restart() }
                }
            }
        }
        Some(Err(e)) => rsx! {
            p { class: "error", "Error loading shopping list: {e}" }
        },
        None => rsx! {
            p { "Loading..." }
        },
    };

    rsx! {
        document::Title { "{title}" }
        {body}
    }
}

fn group_by_section(
    items: &[ShoppingListItemView],
) -> Vec<(Option<GrocerySection>, Vec<ShoppingListItemView>)> {
    let mut buckets: HashMap<Option<GrocerySection>, Vec<ShoppingListItemView>> = HashMap::new();
    for it in items {
        buckets
            .entry(it.grocery_section)
            .or_default()
            .push(it.clone());
    }
    let mut out = Vec::new();
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
    items: Vec<ShoppingListItemView>,
    on_change: EventHandler<()>,
) -> Element {
    // Group rows sharing a display name so duplicate ingredients collapse into
    // one tappable row.
    let mut by_name: Vec<(String, Vec<ShoppingListItemView>)> = Vec::new();
    for it in items {
        let key = it.display_name().to_lowercase();
        if let Some(existing) = by_name.iter_mut().find(|(k, _)| *k == key) {
            existing.1.push(it);
        } else {
            by_name.push((key, vec![it]));
        }
    }
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
fn NameRow(items: Vec<ShoppingListItemView>, on_change: EventHandler<()>) -> Element {
    let display_name = items[0].display_name().to_string();
    let all_checked = items.iter().all(|i| i.checked);
    let qty_text = items
        .iter()
        .map(format_qty_unit)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let item_ids: Vec<ShoppingListItemId> = items.iter().map(|i| i.id).collect();
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
            for id in ids {
                let _ = set_shopping_list_item_checked(id, target).await;
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
                let _ = delete_shopping_list_item(id).await;
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
                class: "icon-button trash shopping-row-delete",
                "aria-label": "Remove",
                title: "Remove",
                disabled: busy(),
                onclick: delete,
                TrashIcon {}
            }
        }
    }
}

fn format_qty_unit(it: &ShoppingListItemView) -> String {
    let qty = it.quantity.map(format_quantity);
    let unit = it.unit.clone().filter(|l| !l.is_empty());
    match (qty, unit) {
        (Some(q), Some(u)) => format!("{q} {u}"),
        (Some(q), None) => q,
        (None, Some(u)) => u,
        (None, None) => String::new(),
    }
}

#[component]
fn AddItemForm(list_id: ShoppingListId, on_added: EventHandler<()>) -> Element {
    let mut expanded = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut qty = use_signal(String::new);
    let mut unit = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        if name.read().trim().is_empty() {
            error.set(Some("Name is required.".into()));
            return;
        }

        submitting.set(true);
        error.set(None);

        let input = ShoppingListItemInput {
            text: name.read().clone(),
            quantity: qty.read().clone(),
            unit: unit.read().clone(),
        };
        let res = add_shopping_list_item(list_id, input).await;
        submitting.set(false);

        match res {
            Ok(_) => {
                name.set(String::new());
                qty.set(String::new());
                unit.set(String::new());
                on_added.call(());
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    };

    if !expanded() {
        return rsx! {
            div { class: "shopping-add-bar",
                button {
                    r#type: "button",
                    class: "primary shopping-add-toggle",
                    onclick: move |_| expanded.set(true),
                    "+ Add item"
                }
            }
        };
    }

    rsx! {
        div { class: "shopping-add-bar",
            form { class: "shopping-add", onsubmit: submit,
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
                    input {
                        placeholder: "Unit",
                        class: "shopping-add-unit",
                        value: unit(),
                        oninput: move |e| unit.set(e.value()),
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
                        class: "primary",
                        disabled: submitting(),
                        if submitting() {
                            "Adding..."
                        } else {
                            "Add"
                        }
                    }
                }
            }
        }
    }
}
