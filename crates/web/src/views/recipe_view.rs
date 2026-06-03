use {
    super::{duration::format_duration, format::format_quantity},
    api::{RecipeDetail, RecipeStepIngredientDetail},
    dioxus::prelude::*,
    pulldown_cmark::{Options, Parser, html},
    ui::{RunningTimersCtx, icons::HourglassIcon, timers},
};

#[component]
pub fn RecipeView(
    detail: RecipeDetail,
    multiplier: f64,
    #[props(default)] meal_key: Option<String>,
) -> Element {
    let timers_ctx = use_context::<RunningTimersCtx>();
    let recipe_name = detail.recipe.name.clone();
    let recipe_slug = detail.recipe.slug.clone();

    rsx! {
        section { class: "recipe",
            if !detail.recipe.source.is_empty() {
                p { class: "source",
                    "Source: "
                    if detail.recipe.source.starts_with("http") {
                        a { href: "{detail.recipe.source}", "{detail.recipe.source}" }
                    } else {
                        span { "{detail.recipe.source}" }
                    }
                }
            }

            table { class: "recipe-steps",
                tbody {
                    for (idx, step) in detail.steps.into_iter().enumerate() {
                        tr { key: "{step.step.id}", id: "step-{idx + 1}",
                            td { class: "ingredients",
                                for ing in step.ingredients.iter() {
                                    div {
                                        key: "{ing.rsi.id}",
                                        class: "ingredient-block",
                                        "{format_ingredient_line(ing, multiplier)}"
                                    }
                                }
                                if let Some(d) = step.step.duration_s {
                                    {
                                        let step_number = (idx + 1) as i64;
                                        let pretty = format_duration(d as i64);
                                        let recipe_name = recipe_name.clone();
                                        let recipe_slug = recipe_slug.clone();
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
                                                        recipe_slug.clone(),
                                                        recipe_name.clone(),
                                                        step_number,
                                                        d as i64,
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
                                div {
                                    class: "instruction-block",
                                    dangerous_inner_html: render_markdown(&step.step.text),
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

fn format_ingredient_line(ing: &RecipeStepIngredientDetail, multiplier: f64) -> String {
    let name = ing.ingredient.name.as_ref();
    let qty = ing.rsi.quantity.map(|q| format_quantity(q * multiplier));
    let unit = ing.rsi.unit.as_ref().filter(|l| !l.is_empty());

    match (qty, unit) {
        (Some(q), Some(u)) => format!("{q} {u} {name}"),
        (Some(q), None) => format!("{q} {name}"),
        (None, Some(u)) => format!("{u} {name}"),
        (None, None) => name.to_string(),
    }
}
