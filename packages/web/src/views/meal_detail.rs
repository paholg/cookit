use api::meals::get_meal;
use api::shopping_lists::create_from_meal;
use dioxus::prelude::*;
use ui::icons::{EditIcon, ListIcon};

use crate::{
    CurrentUserCtx, Route,
    views::{RecipeView, WakeLockToggle},
};

#[component]
pub fn MealDetail(id: i64) -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();
    let meal = use_resource(move || get_meal(id));
    let mut tab = use_signal(|| 0usize);
    let mut making = use_signal(|| false);
    let mut make_error: Signal<Option<String>> = use_signal(|| None);
    let nav = navigator();

    let title = meal
        .cloned()
        .and_then(|m| m.ok())
        .map(|d| d.meal.name)
        .unwrap_or_else(|| "CookIt!".to_string());

    let body = match meal.cloned() {
        Some(Ok(detail)) => {
            let recipe_count = detail.recipes.len();
            let current = tab().min(recipe_count.saturating_sub(1));

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
                                onclick: move |_| async move {
                                    making.set(true);
                                    make_error.set(None);
                                    match create_from_meal(id, authenticated).await {
                                        Ok(new_id) => {
                                            nav.push(Route::ShoppingListDetail { id: new_id });
                                        }
                                        Err(e) => {
                                            make_error.set(Some(e));
                                            making.set(false);
                                        }
                                    }
                                },
                                ListIcon {}
                            }
                            Link {
                                to: Route::MealEdit { id },
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
                                    class: if i == current { "tab active" } else { "tab" },
                                    onclick: move |_| tab.set(i),
                                    "{mr.recipe.recipe.name}"
                                    if (mr.multiplier - 1.0).abs() > f64::EPSILON {
                                        span { class: "tab-mult", " ({format_mult(mr.multiplier)}×)" }
                                    }
                                }
                            }
                        }

                        if let Some(mr) = detail.recipes.get(current) {
                            RecipeView {
                                detail: mr.recipe.clone(),
                                multiplier: mr.multiplier,
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
