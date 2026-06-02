use {
    crate::Route,
    api::{create_shopping_list, create_shopping_list_from_meal, list_meals},
    dioxus::prelude::*,
    ui::ClientOnly,
};

#[component]
pub fn ShoppingListNew() -> Element {
    let nav = navigator();

    let meals = use_server_future(list_meals)?;
    let mut name = use_signal(String::new);
    let mut from_meal_key: Signal<Option<String>> = use_signal(|| None);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        submitting.set(true);
        error.set(None);

        let result = match from_meal_key() {
            Some(meal_key) => create_shopping_list_from_meal(meal_key).await,
            None => {
                let trimmed = name.read().trim().to_string();
                if trimmed.is_empty() {
                    submitting.set(false);
                    error.set(Some("Pick a meal or enter a list name.".into()));
                    return;
                }
                create_shopping_list(trimmed).await
            }
        };

        match result {
            Ok(id) => {
                nav.replace(Route::ShoppingListDetail { id });
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "New shopping list" }
        header { class: "page-header",
            h1 { "New shopping list" }
        }

        form { class: "app-form", onsubmit: submit,

            fieldset {
                legend { "Generate from a meal" }
                match meals.cloned() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        ClientOnly {
                            select {
                                value: from_meal_key().unwrap_or_default(),
                                onchange: move |e| {
                                    let v = e.value();
                                    from_meal_key.set(if v.is_empty() { None } else { Some(v) });
                                },
                                option { value: "", "— Empty list —" }
                                for m in list {
                                    option { value: "{m.slug}", "{m.name}" }
                                }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        p { class: "empty", "No meals to generate from." }
                    },
                    Some(Err(e)) => rsx! {
                        p { class: "error", "Error loading meals: {e}" }
                    },
                    None => rsx! {
                        p { "Loading meals..." }
                    },
                }
            }

            fieldset {
                legend { "Or start an empty list" }
                input {
                    placeholder: "List name",
                    value: name(),
                    disabled: from_meal_key().is_some(),
                    oninput: move |e| name.set(e.value()),
                }
            }

            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }

            button { r#type: "submit", class: "primary", disabled: submitting(),
                if submitting() {
                    "Creating..."
                } else {
                    "Create"
                }
            }
        }
    }
}
