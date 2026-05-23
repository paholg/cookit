use crate::Route;
use api::{RecipeStepIngredient, get_recipe};
use dioxus::prelude::*;
#[component]
pub fn RecipeDetail(id: i64) -> Element {
    let recipe = use_server_future(move || get_recipe(id))?;
    rsx! {
        match recipe.cloned() {
            Some(Ok(detail)) => rsx! {
                article { class: "recipe",
                    header { class: "page-header",
                        h1 { "{detail.recipe.name}" }
                        Link { to: Route::RecipeEdit { id }, class: "button-link", "Edit" }
                    }
                    if let Some(source) = detail.recipe.source.as_deref() {
                        p { class: "source",
                            "Source: "
                            if source.starts_with("http") {
                                a { href: "{source}", "{source}" }
                            } else {
                                span { "{source}" }
                            }
                        }
                    }
                    ol { class: "steps",
                        for step in detail.steps {
                            li { key: "{step.id}",
                                p { class: "instruction", "{step.instruction}" }
                                if !step.ingredients.is_empty() {
                                    ul { class: "ingredients",
                                        for ing in step.ingredients {
                                            li { key: "{ing.ingredient_id}-{ing.position}",
                                                "{format_ingredient_line(&ing)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading recipe: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
        }
    }
}
fn format_quantity(q: f64) -> String {
    if (q.fract()).abs() < f64::EPSILON {
        format!("{}", q as i64)
    } else {
        format!("{q}")
    }
}
fn format_ingredient_line(ing: &RecipeStepIngredient) -> String {
    let qty = format_quantity(ing.quantity);
    let unit = ing.unit.label();
    if unit.is_empty() {
        format!("{qty} {}", ing.ingredient_name)
    } else {
        format!("{qty} {unit} {}", ing.ingredient_name)
    }
}
