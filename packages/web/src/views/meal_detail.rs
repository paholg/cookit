use crate::{CurrentUserCtx, Route};
use api::meals::{delete_meal, get_meal};
use dioxus::prelude::*;
use types::{CurrentUser, RecipeDetail, RecipeStepIngredient};

fn can_modify(user: &Option<CurrentUser>, owner_id: Option<i64>) -> bool {
    match owner_id {
        // Local-storage meals have no owner — anyone viewing them owns them.
        None => true,
        Some(owner) => match user {
            Some(u) => u.is_admin || u.id == owner,
            None => false,
        },
    }
}

#[component]
pub fn MealDetail(id: i64) -> Element {
    let meal = use_resource(move || get_meal(id));
    let mut tab = use_signal(|| 0usize);
    let user = use_context::<CurrentUserCtx>();
    let nav = use_navigator();
    let mut deleting = use_signal(|| false);
    let mut delete_error: Signal<Option<String>> = use_signal(|| None);

    let title = meal
        .cloned()
        .and_then(|m| m.ok())
        .map(|d| d.meal.name)
        .unwrap_or_else(|| "CookIt!".to_string());

    let body = match meal.cloned() {
        Some(Ok(detail)) => {
            let recipe_count = detail.recipes.len();
            let current = tab().min(recipe_count.saturating_sub(1));
            let modifiable = can_modify(&user.read(), detail.meal.user_id);
            rsx! {
                article { class: "meal",
                    header { class: "page-header",
                        h1 { "{detail.meal.name}" }
                        if modifiable {
                            Link { to: Route::MealEdit { id }, class: "button-link", "Edit" }
                            button {
                                r#type: "button",
                                class: "button-link danger",
                                disabled: deleting(),
                                onclick: move |_| {
                                    if deleting() { return; }
                                    deleting.set(true);
                                    delete_error.set(None);
                                    spawn(async move {
                                        match delete_meal(id).await {
                                            Ok(()) => { nav.push(Route::MealList {}); }
                                            Err(e) => {
                                                delete_error.set(Some(e.to_string()));
                                                deleting.set(false);
                                            }
                                        }
                                    });
                                },
                                if deleting() { "Deleting..." } else { "Delete" }
                            }
                        }
                    }
                    if let Some(err) = delete_error() {
                        p { class: "error", "Delete failed: {err}" }
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
                            RecipeCookingView {
                                detail: mr.recipe.clone(),
                                multiplier: mr.multiplier,
                            }
                        }
                    }
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                p { class: "error", "Error loading meal: {e}" }
            }
        }
        None => {
            rsx! {
                p { "Loading..." }
            }
        }
    };

    rsx! {
        document::Title { "{title}" }
        {body}
    }
}

#[component]
fn RecipeCookingView(detail: RecipeDetail, multiplier: f64) -> Element {
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
            ol { class: "steps",
                for step in detail.steps {
                    li { key: "{step.id}",
                        p { class: "instruction", "{step.instruction}" }
                        if !step.ingredients.is_empty() {
                            ul { class: "ingredients",
                                for ing in step.ingredients {
                                    li { key: "{ing.ingredient_id}-{ing.position}",
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
    let unit = ing.unit.label();
    if unit.is_empty() {
        format!("{qty} {}", ing.ingredient_name)
    } else {
        format!("{qty} {unit} {}", ing.ingredient_name)
    }
}
