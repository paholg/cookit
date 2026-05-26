use dioxus::prelude::*;
use pulldown_cmark::{Options, Parser, html};
use types::{RecipeDetail, RecipeStepIngredient};
use ui::icons::HourglassIcon;

use super::duration::format_duration;
use super::format::format_quantity;
use crate::timers::{self, RunningTimersCtx};

#[component]
pub fn RecipeView(
    detail: RecipeDetail,
    multiplier: f64,
    /// Set when this view is rendered inside a `MealDetail`, so a timer
    /// started here knows which meal it belongs to. `None` on the standalone
    /// recipe page.
    #[props(default)]
    meal_key: Option<String>,
) -> Element {
    let timers_ctx = use_context::<RunningTimersCtx>();
    let recipe_name = detail.recipe.name.clone();
    let recipe_key = detail.recipe.key.clone();

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
                    for (idx, step) in detail.steps.into_iter().enumerate() {
                        tr { key: "{step.id}",
                            id: "step-{idx + 1}",
                            td { class: "ingredients",
                                for ing in step.ingredients.iter() {
                                    div {
                                        key: "{ing.id}",
                                        class: "ingredient-block",
                                        "{format_ingredient_line(ing, multiplier)}"
                                    }
                                }
                                if let Some(d) = step.duration_seconds {
                                    {
                                        let step_number = (idx + 1) as i64;
                                        let pretty = format_duration(d);
                                        let recipe_name = recipe_name.clone();
                                        let recipe_key = recipe_key.clone();
                                        let meal_key = meal_key.clone();
                                        rsx! {
                                            button {
                                                r#type: "button",
                                                class: "step-timer-start",
                                                "aria-label": "Start {pretty} timer for {recipe_name} step {step_number}",
                                                title: "Start {pretty} timer",
                                                onclick: move |_| {
                                                    timers::start_timer(
                                                        timers_ctx,
                                                        meal_key.clone(),
                                                        recipe_key.clone(),
                                                        recipe_name.clone(),
                                                        step_number,
                                                        d,
                                                    );
                                                },
                                                HourglassIcon {}
                                                span { class: "step-timer-duration", "{pretty}" }
                                            }
                                        }
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
