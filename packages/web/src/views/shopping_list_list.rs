use crate::{CurrentUserCtx, Route};
use api::shopping_lists::{delete_shopping_list, list_shopping_lists};
use dioxus::prelude::*;
use types::ShoppingList;
use ui::icons::TrashIcon;

#[component]
pub fn ShoppingListList() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();

    let mut lists = use_resource(move || list_shopping_lists(authenticated));

    rsx! {
        document::Title { "Shopping" }
        header { class: "page-header",
            h1 { "Shopping lists" }
            Link { to: Route::ShoppingListNew {}, class: "button", "+ New list" }
        }

        match lists.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No shopping lists yet." }
            },
            Some(Ok(list)) => rsx! {
                ul { class: "recipe-list",
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

    rsx! {
        li {
            div { class: "meal-row-main",
                Link { to: Route::ShoppingListDetail { id }, "{list.name}" }
                button {
                    r#type: "button",
                    class: "icon-button",
                    "aria-label": "Delete shopping list",
                    title: "Delete shopping list",
                    disabled: deleting(),
                    onclick: move |_| async move {
                        let confirmed = document::eval(
                            "return confirm('Delete this shopping list? This cannot be undone.')",
                        )
                            .join::<bool>()
                            .await
                            .unwrap_or(false);
                        if !confirmed { return; }
                        deleting.set(true);
                        match delete_shopping_list(id).await {
                            Ok(()) => on_deleted.call(()),
                            Err(e) => error.set(Some(e)),
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
