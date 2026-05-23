use crate::Route;
use api::{RecipeDetail, RecipeStepIngredient, get_meal};
use dioxus::prelude::*;

#[component]
pub fn MealDetail(id: i64) -> Element {
    let meal = use_server_future(move || get_meal(id))?;
    let mut tab = use_signal(|| 0usize);

    match meal.cloned() {
        Some(Ok(detail)) => {
            let recipe_count = detail.recipes.len();
            let current = tab().min(recipe_count.saturating_sub(1));
            rsx! {
                article {
                    class: "meal",
                    header {
                        class: "page-header",
                        h1 { "{detail.meal.name}" }
                        Link {
                            to: Route::MealEdit { id },
                            class: "button-link",
                            "Edit"
                        }
                    }

                    if recipe_count == 0 {
                        p { class: "empty", "This meal has no recipes yet." }
                    } else {
                        nav {
                            class: "meal-tabs",
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
                            RecipeCookingView {
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
    }
}

#[component]
fn RecipeCookingView(detail: RecipeDetail, multiplier: f64) -> Element {
    rsx! {
        section {
            class: "recipe",
            if let Some(source) = detail.recipe.source.as_deref() {
                p {
                    class: "source",
                    "Source: "
                    if source.starts_with("http") {
                        a { href: "{source}", "{source}" }
                    } else {
                        span { "{source}" }
                    }
                }
            }

            ol {
                class: "steps",
                for step in detail.steps {
                    li {
                        key: "{step.id}",
                        p { class: "instruction", "{step.instruction}" }
                        if !step.ingredients.is_empty() {
                            ul {
                                class: "ingredients",
                                for ing in step.ingredients {
                                    li {
                                        key: "{ing.ingredient_id}-{ing.position}",
                                        "{scaled_line(&ing, multiplier)}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_mult(m: f64) -> String {
    if (m.fract()).abs() < f64::EPSILON {
        format!("{}", m as i64)
    } else {
        format!("{m}")
    }
}

fn format_quantity(q: f64) -> String {
    if (q.fract()).abs() < f64::EPSILON {
        format!("{}", q as i64)
    } else {
        // Trim to 2 decimals for the multiplied case so "0.5 × 2.5" doesn't render as "1.25" with 14 digits.
        let rounded = (q * 100.0).round() / 100.0;
        if (rounded.fract()).abs() < f64::EPSILON {
            format!("{}", rounded as i64)
        } else {
            format!("{rounded}")
        }
    }
}

fn scaled_line(ing: &RecipeStepIngredient, multiplier: f64) -> String {
    let qty = format_quantity(ing.quantity * multiplier);
    if ing.unit.is_empty() {
        format!("{qty} {}", ing.ingredient_name)
    } else {
        format!("{qty} {} {}", ing.unit, ing.ingredient_name)
    }
}
