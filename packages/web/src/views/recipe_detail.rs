use crate::Route;
use api::{RecipeStepIngredient, get_recipe};
use dioxus::prelude::*;
use pulldown_cmark::{Options, Parser, html};

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
                    table { class: "recipe-steps",
                        tbody {
                            for step in detail.steps {
                                tr { key: "{step.id}",
                                    td { class: "instruction",
                                        div { dangerous_inner_html: render_markdown(&step.instruction) }
                                    }
                                    td { class: "ingredients",
                                        for (i, ing) in step.ingredients.iter().enumerate() {
                                            div { key: "{ing.ingredient_id}-{ing.position}-{i}",
                                                "• {format_ingredient_line(ing)}"
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
