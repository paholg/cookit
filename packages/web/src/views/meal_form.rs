use crate::Route;
use api::{create_meal, list_recipes, update_meal};
use dioxus::prelude::*;
use types::{MealDetail, NewMeal, NewMealRecipe, Recipe};
#[derive(Default, Clone, PartialEq)]
pub struct MealRecipeDraft {
    pub recipe_id: Option<i64>,
    pub multiplier: String,
}
#[derive(Default, Clone, PartialEq)]
pub struct MealDraft {
    pub name: String,
    pub recipes: Vec<MealRecipeDraft>,
}
impl MealDraft {
    pub fn empty() -> Self {
        Self {
            recipes: vec![MealRecipeDraft {
                recipe_id: None,
                multiplier: "1".into(),
            }],
            ..Self::default()
        }
    }
    pub fn from_detail(detail: MealDetail) -> Self {
        Self {
            name: detail.meal.name,
            recipes: detail
                .recipes
                .into_iter()
                .map(|mr| MealRecipeDraft {
                    recipe_id: Some(mr.recipe.recipe.id),
                    multiplier: format_mult(mr.multiplier),
                })
                .collect(),
        }
    }
    fn to_payload(&self) -> Result<NewMeal, String> {
        let mut recipes = Vec::with_capacity(self.recipes.len());
        for (idx, r) in self.recipes.iter().enumerate() {
            let Some(recipe_id) = r.recipe_id else {
                return Err(format!("row {}: pick a recipe", idx + 1));
            };
            let m_text = r.multiplier.trim();
            let multiplier: f64 = if m_text.is_empty() {
                return Err(format!("row {}: multiplier is required", idx + 1));
            } else {
                m_text
                    .parse()
                    .map_err(|_| format!("row {}: `{m_text}` is not a valid number", idx + 1))?
            };
            recipes.push(NewMealRecipe {
                recipe_id,
                multiplier,
            });
        }
        Ok(NewMeal {
            name: self.name.clone(),
            recipes,
        })
    }
}
fn format_mult(m: f64) -> String {
    if (m.fract()).abs() < f64::EPSILON {
        format!("{}", m as i64)
    } else {
        format!("{m}")
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum MealFormMode {
    Create,
    Edit { id: i64 },
}
#[component]
pub fn MealForm(initial: MealDraft, mode: MealFormMode) -> Element {
    let mut draft = use_signal(|| initial.clone());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);
    let nav = use_navigator();
    let recipes = use_server_future(list_recipes)?;
    let available: Vec<Recipe> = match recipes.cloned() {
        Some(Ok(list)) => list,
        _ => Vec::new(),
    };
    let submit = move |e: FormEvent| {
        e.prevent_default();
        if submitting() {
            return;
        }
        let payload = match draft.read().to_payload() {
            Ok(p) => p,
            Err(msg) => {
                error.set(Some(msg));
                return;
            }
        };
        submitting.set(true);
        error.set(None);
        spawn(async move {
            let result: Result<i64, String> = match mode {
                MealFormMode::Create => create_meal(payload).await.map_err(|e| e.to_string()),
                MealFormMode::Edit { id } => match update_meal(id, payload).await {
                    Ok(()) => Ok(id),
                    Err(e) => Err(e.to_string()),
                },
            };
            match result {
                Ok(id) => {
                    nav.push(Route::MealDetail { id });
                }
                Err(msg) => {
                    submitting.set(false);
                    error.set(Some(msg));
                }
            }
        });
    };
    let title = match mode {
        MealFormMode::Create => "New meal",
        MealFormMode::Edit { .. } => "Edit meal",
    };
    let submit_label_idle = match mode {
        MealFormMode::Create => "Save meal",
        MealFormMode::Edit { .. } => "Save changes",
    };
    rsx! {
        header { class: "page-header",
            h1 { "{title}" }
        }
        if available.is_empty() {
            p { class: "empty",
                "You don't have any recipes yet. "
                Link { to: Route::RecipeNew
                                                                                                                                                    {}, "Create one" }
                " before building a meal."
            }
        }
        form { class: "recipe-form", onsubmit: submit,
            label {
                "Name"
                input {
                    r#type: "text",
                    required: true,
                    value: "{draft.read().name}",
                    oninput: move |e| draft.write().name = e.value(),
                }
            }
            h2 { "Recipes" }
            {
                let count = draft.read().recipes.len();
                let avail = available.clone();
                rsx! {
                    for idx in 0..count {
                        MealRecipeRow {
                            key: "{idx}",
                            idx,
                            draft,
                            recipes: avail.clone(),
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                disabled: available.is_empty(),
                onclick: move |_| {
                    draft
                        .write()
                        .recipes
                        .push(MealRecipeDraft {
                            recipe_id: None,
                            multiplier: "1".into(),
                        })
                },
                "+ Add recipe"
            }
            if let Some(err) = error.read().clone() {
                p { class: "error", "{err}" }
            }
            div { class: "form-actions",
                button {
                    r#type: "submit",
                    class: "primary",
                    disabled: submitting(),
                    if submitting() {
                        "Saving..."
                    } else {
                        "{submit_label_idle}"
                    }
                }
                if let MealFormMode::Edit { id } = mode {
                    Link { to: Route::MealDetail { id }, class: "button-link", "Cancel" }
                }
            }
        }
    }
}
#[component]
fn MealRecipeRow(idx: usize, draft: Signal<MealDraft>, recipes: Vec<Recipe>) -> Element {
    let row = draft.read().recipes.get(idx).cloned().unwrap_or_default();
    let selected = row.recipe_id.map(|i| i.to_string()).unwrap_or_default();
    rsx! {
        div { class: "meal-row",
            select {
                value: "{selected}",
                oninput: move |e| {
                    let v = e.value();
                    let id: Option<i64> = v.parse().ok();
                    let mut d = draft.write();
                    if let Some(r) = d.recipes.get_mut(idx) {
                        r.recipe_id = id;
                    }
                },
                option { value: "", "— pick a recipe —" }
                for r in recipes.iter() {
                    option {
                        value: "{r.id}",
                        selected: row.recipe_id == Some(r.id),
                        "{r.name}"
                    }
                }
            }
            input {
                r#type: "text",
                inputmode: "decimal",
                placeholder: "×",
                value: "{row.multiplier}",
                oninput: move |e| {
                    let mut d = draft.write();
                    if let Some(r) = d.recipes.get_mut(idx) {
                        r.multiplier = e.value();
                    }
                },
            }
            button {
                r#type: "button",
                class: "danger small",
                onclick: move |_| {
                    let mut d = draft.write();
                    if idx < d.recipes.len() {
                        d.recipes.remove(idx);
                    }
                },
                "×"
            }
        }
    }
}
