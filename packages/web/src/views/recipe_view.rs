use dioxus::prelude::*;
use pulldown_cmark::{Options, Parser, html};
use types::{RecipeDetail, RecipeStepIngredient};

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
                    for step in detail.steps {
                        tr { key: "{step.id}",
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

fn format_quantity(q: f64) -> String {
    if q.fract().abs() < f64::EPSILON {
        return format!("{}", q as i64);
    }

    let rounded = (q * 100.0).round() / 100.0;
    if rounded.fract().abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

fn format_ingredient_line(ing: &RecipeStepIngredient, multiplier: f64) -> String {
    let qty = format_quantity(ing.quantity * multiplier);
    let unit = ing.unit.label();

    if unit.is_empty() {
        format!("{qty} {}", ing.ingredient_name)
    } else {
        format!("{qty} {unit} {}", ing.ingredient_name)
    }
}
