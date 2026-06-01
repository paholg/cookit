use {
    crate::{
        CurrentUserCtx, Route,
        views::{RecipeView, WakeLockToggle},
    },
    api::{meals::get_meal, shopping_lists::create_from_meal},
    dioxus::prelude::*,
    ui::icons::{EditIcon, ListIcon},
};

#[component]
pub fn MealDetail(meal_key: String, tab: Option<String>) -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();
    let meal = {
        let meal_key = meal_key.clone();
        use_resource(move || get_meal(meal_key.clone()))
    };
    let mut making = use_signal(|| false);
    let mut make_error: Signal<Option<String>> = use_signal(|| None);
    let nav = navigator();

    use_effect(move || {
        let ready = meal.read().as_ref().map(|r| r.is_ok()).unwrap_or(false);
        if ready {
            document::eval(
                r#"
                requestAnimationFrame(() => {
                    const h = window.location.hash;
                    if (!h) return;
                    try {
                        const el = document.querySelector(h);
                        if (el) el.scrollIntoView({ block: 'start' });
                    } catch (e) {}
                });
                "#,
            );
        }
    });

    let title = meal
        .cloned()
        .and_then(|m| m.ok())
        .map(|d| d.name)
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
                        .position(|mr| mr.recipe_detail.recipe.slug == slug)
                })
                .unwrap_or(0);

            let oc_meal_key = meal_key.clone();
            let onclick = move |_| {
                let meal_key = oc_meal_key.clone();
                async move {
                    making.set(true);
                    make_error.set(None);
                    match create_from_meal(meal_key, authenticated).await {
                        Ok(new_id) => {
                            nav.push(Route::ShoppingListDetail { id: new_id });
                        }
                        Err(e) => {
                            make_error.set(Some(e));
                            making.set(false);
                        }
                    }
                }
            };

            rsx! {
                article { class: "meal",
                    header { class: "page-header",
                        h1 { "{detail.name}" }
                        div { class: "page-header-actions",
                            WakeLockToggle {}
                            button {
                                r#type: "button",
                                class: "icon-button",
                                "aria-label": "Make shopping list",
                                title: "Make shopping list",
                                disabled: making(),
                                onclick,
                                ListIcon {}
                            }
                            Link {
                                to: Route::MealEdit {
                                    meal_key: meal_key.clone(),
                                },
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
                                    key: "{mr.recipe_detail.recipe.id}",
                                    r#type: "button",
                                    class: if i == current { "tab active" } else { "tab" },
                                    onclick: {
                                        let recipe_slug = mr.recipe_detail.recipe.slug.clone();
                                        let meal_key = meal_key.clone();
                                        move |_| {
                                            nav.replace(Route::MealDetail {
                                                meal_key: meal_key.clone(),
                                                tab: Some(recipe_slug.clone()),
                                            });
                                        }
                                    },
                                    "{mr.recipe_detail.recipe.name}"
                                    if (mr.multiplier - 1.0).abs() > f64::EPSILON {
                                        span { class: "tab-mult", " ({format_mult(mr.multiplier)}×)" }
                                    }
                                }
                            }
                        }

                        if let Some(mr) = detail.recipes.get(current) {
                            RecipeView {
                                detail: mr.recipe_detail.clone(),
                                multiplier: mr.multiplier,
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
