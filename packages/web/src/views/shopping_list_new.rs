use crate::{CurrentUserCtx, Route};
use api::meals::list_meals;
use api::shopping_lists::{create_from_meal, create_shopping_list};
use dioxus::prelude::*;
use types::NewShoppingList;

#[component]
pub fn ShoppingListNew() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();
    let nav = navigator();

    let meals = use_resource(move || list_meals(authenticated));
    let mut name = use_signal(String::new);
    let mut from_meal_id: Signal<Option<i64>> = use_signal(|| None);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        submitting.set(true);
        error.set(None);

        let result = match from_meal_id() {
            Some(meal_id) => create_from_meal(meal_id, authenticated).await,
            None => {
                let trimmed = name.read().trim().to_string();
                if trimmed.is_empty() {
                    submitting.set(false);
                    error.set(Some("Pick a meal or enter a list name.".into()));
                    return;
                }
                create_shopping_list(
                    NewShoppingList {
                        name: trimmed,
                        items: Vec::new(),
                    },
                    authenticated,
                )
                .await
            }
        };

        match result {
            Ok(id) => {
                nav.replace(Route::ShoppingListDetail { id });
            }
            Err(e) => {
                error.set(Some(e));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "New shopping list" }
        header { class: "page-header", h1 { "New shopping list" } }

        form {
            class: "meal-form",
            onsubmit: submit,

            fieldset {
                legend { "Generate from a meal" }
                match meals.cloned() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        select {
                            value: from_meal_id().map(|id| id.to_string()).unwrap_or_default(),
                            onchange: move |e| {
                                let v = e.value();
                                from_meal_id.set(v.parse::<i64>().ok());
                            },
                            option { value: "", "— Empty list —" }
                            for m in list {
                                option { value: "{m.id}", "{m.name}" }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        p { class: "empty", "No meals to generate from." }
                    },
                    Some(Err(e)) => rsx! { p { class: "error", "Error loading meals: {e}" } },
                    None => rsx! { p { "Loading meals..." } },
                }
            }

            fieldset {
                legend { "Or start an empty list" }
                input {
                    placeholder: "List name",
                    value: name(),
                    disabled: from_meal_id().is_some(),
                    oninput: move |e| name.set(e.value()),
                }
            }

            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }

            button {
                r#type: "submit",
                class: "button",
                disabled: submitting(),
                if submitting() { "Creating..." } else { "Create" }
            }
        }
    }
}
