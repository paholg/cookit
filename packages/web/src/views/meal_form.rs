use crate::{CurrentUserCtx, Route, draft_id::DraftId};

use api::{
    list_recipes,
    meals::{create_meal, delete_meal, update_meal},
};
use dioxus::prelude::*;
use types::{MealDetail, NewMeal, NewMealRecipe, Recipe};
use ui::icons::TrashIcon;

#[derive(Default, Clone, PartialEq)]
pub struct MealRecipeDraft {
    pub id: DraftId,
    pub recipe_id: Option<i64>,
    pub multiplier: String,
}

#[derive(Default, Clone, PartialEq)]
pub struct MealDraft {
    pub name: String,
    pub recipes: Vec<MealRecipeDraft>,
    next_row_id: i64,
}

impl MealDraft {
    fn alloc_row_id(&mut self) -> DraftId {
        let id = DraftId::New(self.next_row_id);
        self.next_row_id += 1;
        id
    }

    pub fn empty() -> Self {
        let mut d = Self::default();
        let id = d.alloc_row_id();
        d.recipes.push(MealRecipeDraft {
            id,
            recipe_id: None,
            multiplier: "1".into(),
        });
        d
    }

    pub fn from_detail(detail: MealDetail) -> Self {
        Self {
            name: detail.meal.name,
            recipes: detail
                .recipes
                .into_iter()
                .map(|mr| MealRecipeDraft {
                    id: mr.recipe.recipe.id.into(),
                    recipe_id: Some(mr.recipe.recipe.id),
                    multiplier: format_mult(mr.multiplier),
                })
                .collect(),
            next_row_id: 0,
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

fn all_recipes_used(draft: &MealDraft, available: &[Recipe]) -> bool {
    let picked: std::collections::HashSet<i64> = draft
        .recipes
        .iter()
        .filter_map(|r| r.recipe_id)
        .collect();
    !available.is_empty() && available.iter().all(|r| picked.contains(&r.id))
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
    let mut deleting = use_signal(|| false);
    let nav = use_navigator();
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();
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
                MealFormMode::Create => create_meal(payload, authenticated).await,
                MealFormMode::Edit { id } => update_meal(id, payload).await.map(|()| id),
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
            if let MealFormMode::Edit { id } = mode {
                button {
                    r#type: "button",
                    class: "icon-button",
                    "aria-label": "Delete meal",
                    title: "Delete meal",
                    disabled: deleting() || submitting(),
                    onclick: move |_| {
                        if deleting() { return; }
                        spawn(async move {
                            let confirmed = document::eval(
                                "return confirm('Delete this meal? This cannot be undone.')",
                            )
                                .join::<bool>()
                                .await
                                .unwrap_or(false);
                            if !confirmed { return; }

                            deleting.set(true);
                            error.set(None);
                            match delete_meal(id).await {
                                Ok(()) => {
                                    nav.push(Route::MealList {});
                                }
                                Err(msg) => {
                                    error.set(Some(msg));
                                    deleting.set(false);
                                }
                            }
                        });
                    },
                    TrashIcon {}
                }
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
                let rows: Vec<(DraftId, MealRecipeDraft)> = draft
                    .read()
                    .recipes
                    .iter()
                    .map(|r| (r.id, r.clone()))
                    .collect();
                let avail = available.clone();
                rsx! {
                    for (row_id, row) in rows.into_iter() {
                        MealRecipeRow {
                            key: "{row_id}",
                            row_id,
                            row,
                            draft,
                            recipes: avail.clone(),
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                disabled: available.is_empty() || all_recipes_used(&draft.read(), &available),
                onclick: move |_| {
                    let mut d = draft.write();
                    let id = d.alloc_row_id();
                    d.recipes.push(MealRecipeDraft {
                        id,
                        recipe_id: None,
                        multiplier: "1".into(),
                    });
                },
                "+ Add recipe"
            }
            if let Some(err) = error.read().clone() {
                p { class: "error", "{err}" }
            }
            div { class: "form-actions",
                if let MealFormMode::Edit { id } = mode {
                    Link { to: Route::MealDetail { id }, class: "button-link", "Cancel" }
                }
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
            }
        }
    }
}
#[component]
fn MealRecipeRow(
    row_id: DraftId,
    row: MealRecipeDraft,
    draft: Signal<MealDraft>,
    recipes: Vec<Recipe>,
) -> Element {
    let selected = row.recipe_id.map(|i| i.to_string()).unwrap_or_default();

    let used_by_others: std::collections::HashSet<i64> = draft
        .read()
        .recipes
        .iter()
        .filter(|r| r.id != row_id)
        .filter_map(|r| r.recipe_id)
        .collect();

    rsx! {
        div { class: "meal-row",
            select {
                value: "{selected}",
                oninput: move |e| {
                    let v = e.value();
                    let id: Option<i64> = v.parse().ok();
                    let mut d = draft.write();
                    if let Some(r) = d.recipes.iter_mut().find(|r| r.id == row_id) {
                        r.recipe_id = id;
                    }
                },
                option { value: "", "— pick a recipe —" }
                for r in recipes.iter().filter(|r| !used_by_others.contains(&r.id)) {
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
                    if let Some(r) = d.recipes.iter_mut().find(|r| r.id == row_id) {
                        r.multiplier = e.value();
                    }
                },
            }
            button {
                r#type: "button",
                class: "icon-button",
                "aria-label": "Remove recipe",
                title: "Remove recipe",
                onclick: move |_| {
                    draft.write().recipes.retain(|r| r.id != row_id);
                },
                TrashIcon {}
            }
        }
    }
}
