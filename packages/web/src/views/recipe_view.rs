use dioxus::prelude::*;
use pulldown_cmark::{Options, Parser, html};
use types::{RecipeDetail, RecipeStepIngredient};

use super::format::format_quantity;

#[component]
pub fn RecipeView(detail: RecipeDetail, multiplier: f64) -> Element {
    rsx! {
        section { class: "recipe",
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

            table { class: "recipe-steps",
                tbody {
                    for (i, step) in detail.steps.into_iter().enumerate() {
                        tr {
                            key: "{step.id}",
                            id: "step-{i + 1}",
                            td { class: "ingredients",
                                for ing in step.ingredients.iter() {
                                    div {
                                        key: "{ing.id}",
                                        class: "ingredient-block",
                                        "{format_ingredient_line(ing, multiplier)}"
                                    }
                                }
                            }
                            td { class: "instruction",
                                for instr in step.instructions {
                                    div {
                                        key: "{instr.id}",
                                        class: "instruction-block",
                                        dangerous_inner_html: render_markdown(&instr.text),
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

fn render_markdown(src: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(src, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn format_ingredient_line(ing: &RecipeStepIngredient, multiplier: f64) -> String {
    let qty = ing.quantity.map(|q| format_quantity(q * multiplier));
    let unit = ing
        .unit
        .as_ref()
        .map(|u| u.label())
        .filter(|l| !l.is_empty());

    match (qty, unit) {
        (Some(q), Some(u)) => format!("{q} {u} {}", ing.ingredient_name),
        (Some(q), None) => format!("{q} {}", ing.ingredient_name),
        (None, Some(u)) => format!("{u} {}", ing.ingredient_name),
        (None, None) => ing.ingredient_name.clone(),
    }
}
