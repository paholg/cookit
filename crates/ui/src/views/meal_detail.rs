use {
    crate::{
        RecipeView, Route, WakeLockToggle,
        client::client,
        icons::{EditIcon, ListIcon},
    },
    api::{create_shopping_list_from_meal, get_meal},
    dioxus::prelude::*,
};

#[component]
pub fn MealDetail(meal_key: String, tab: Option<String>) -> Element {
    let meal = {
        let meal_key = meal_key.clone();
        use_server_future(move || get_meal(meal_key.clone()))?
    };

    // After the meal loads, honor a `#step-N` hash by scrolling it into view.
    use_effect(move || {
        let ready = meal.read().as_ref().map(|r| r.is_ok()).unwrap_or(false);
        if ready {
            client().scroll_to_hash();
        }
    });

    let mut making = use_signal(|| false);
    let mut make_error: Signal<Option<String>> = use_signal(|| None);

    let title = meal
        .cloned()
        .and_then(|m| m.ok())
        .map(|d| d.meal.name)
        .unwrap_or_else(|| "CookIt!".to_string());

    let body = match meal.cloned() {
        Some(Ok(detail)) => {
            let recipe_count = detail.recipes.len();
            let current = tab
                .as_deref()
                .and_then(|slug| {
                    detail
                        .recipes
                        .iter()
                        .position(|mr| mr.recipe.recipe.slug == slug)
                })
                .unwrap_or(0);

            let make_list = {
                let meal_key = meal_key.clone();
                move |_| {
                    let meal_key = meal_key.clone();
                    async move {
                        making.set(true);
                        make_error.set(None);
                        match create_shopping_list_from_meal(meal_key).await {
                            Ok(new_id) => {
                                navigator().push(Route::ShoppingListDetail { id: new_id });
                            }
                            Err(e) => {
                                make_error.set(Some(e.to_string()));
                                making.set(false);
                            }
                        }
                    }
                }
            };

            rsx! {
                article { class: "meal",
                    header { class: "page-header",
                        h1 { "{detail.meal.name}" }
                        div { class: "page-header-actions",
                            WakeLockToggle {}
                            button {
                                r#type: "button",
                                class: "icon-button",
                                "aria-label": "Make shopping list",
                                title: "Make shopping list",
                                disabled: making(),
                                onclick: make_list,
                                ListIcon {}
                            }
                            Link {
                                to: Route::MealEdit { meal_key: meal_key.clone() },
                                button {
                                    r#type: "button",
                                    class: "icon-button",
                                    "aria-label": "Edit meal",
                                    title: "Edit meal",
                                    EditIcon {}
                                }
                            }
                        }
                    }

                    if let Some(e) = make_error() {
                        p { class: "error", "{e}" }
                    }

                    if recipe_count == 0 {
                        p { class: "empty", "This meal has no recipes yet." }
                    } else {
                        nav { class: "meal-tabs",
                            for (i, mr) in detail.recipes.iter().enumerate() {
                                button {
                                    key: "{mr.recipe.recipe.id}",
                                    r#type: "button",
                                    class: if i == current { "primary" } else { "" },
                                    onclick: {
                                        let recipe_slug = mr.recipe.recipe.slug.clone();
                                        let meal_key = meal_key.clone();
                                        move |_| {
                                            navigator().replace(Route::MealDetail {
                                                meal_key: meal_key.clone(),
                                                tab: Some(recipe_slug.clone()),
                                            });
                                        }
                                    },
                                    "{mr.recipe.recipe.name}"
                                    if (mr.meal_recipe.multiplier - 1.0).abs() > f64::EPSILON {
                                        span { class: "tab-mult", " ({format_mult(mr.meal_recipe.multiplier)}×)" }
                                    }
                                }
                            }
                        }

                        if let Some(mr) = detail.recipes.get(current) {
                            RecipeView {
                                detail: mr.recipe.clone(),
                                multiplier: mr.meal_recipe.multiplier,
                                meal_key: meal_key.clone(),
                            }
                        }
                    }
                }
            }
        }
        Some(Err(e)) => rsx! {
            p { class: "error", "Error loading meal: {e}" }
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

fn format_mult(m: f64) -> String {
    if m.fract().abs() < f64::EPSILON {
        format!("{}", m as i64)
    } else {
        format!("{m}")
    }
}
