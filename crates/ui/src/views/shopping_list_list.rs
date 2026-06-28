use {
    crate::{Route, icons::TrashIcon, use_confirm},
    api::{ShoppingList, delete_shopping_list, list_shopping_lists, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn ShoppingListList() -> Element {
    let mut lists = use_server_future(list_shopping_lists)?;

    rsx! {
        document::Title { "{page_title(\"Shopping\")}" }
        header { class: "page-header",
            h1 { "Shopping lists" }
            Link { to: Route::ShoppingListNew {}, class: "button primary", "+ New list" }
        }

        match lists.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No shopping lists yet." }
            },
            Some(Ok(list)) => rsx! {
                ul { class: "card-list",
                    for sl in list {
                        ShoppingListRow {
                            key: "{sl.id}",
                            list: sl,
                            on_deleted: move |_| lists.restart(),
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading shopping lists: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
        }
    }
}

#[component]
fn ShoppingListRow(list: ShoppingList, on_deleted: EventHandler<()>) -> Element {
    let id = list.id;
    let mut deleting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let confirm = use_confirm();

    rsx! {
        li {
            div { class: "card-row",
                Link { to: Route::ShoppingListDetail { id }, "{list.name}" }
                button {
                    r#type: "button",
                    class: "icon-button trash",
                    "aria-label": "Delete shopping list",
                    title: "Delete shopping list",
                    disabled: deleting(),
                    onclick: move |_| async move {
                        let confirmed = confirm
                            .show("Delete this shopping list? This cannot be undone.")
                            .await;
                        if !confirmed {
                            return;
                        }
                        deleting.set(true);
                        match delete_shopping_list(id).await {
                            Ok(()) => on_deleted.call(()),
                            Err(e) => error.set(Some(e.to_string())),
                        }
                        deleting.set(false);
                    },
                    TrashIcon {}
                }
            }
            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }
        }
    }
}
