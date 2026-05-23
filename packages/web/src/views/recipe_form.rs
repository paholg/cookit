use crate::Route;
use api::{
    MassUnit, NewRecipe, NewStep, NewStepIngredient, RecipeDetail, UnitKind, VolumeUnit,
    create_recipe, list_ingredients, update_recipe,
};
use dioxus::prelude::*;
use std::str::FromStr;
use strum::IntoEnumIterator;

const MASS_LIST_ID: &str = "mass-units";
const VOLUME_LIST_ID: &str = "volume-units";
const INGREDIENT_LIST_ID: &str = "ingredient-names";

#[derive(Default, Clone, PartialEq)]
pub struct IngDraft {
    pub name: String,
    pub quantity: String,
    pub unit_kind: Option<UnitKind>,
    pub unit: String,
}

#[derive(Default, Clone, PartialEq)]
pub struct StepDraft {
    pub instruction: String,
    pub ingredients: Vec<IngDraft>,
}

#[derive(Default, Clone, PartialEq)]
pub struct RecipeDraft {
    pub name: String,
    pub source: String,
    pub steps: Vec<StepDraft>,
}

impl RecipeDraft {
    pub fn empty() -> Self {
        Self {
            steps: vec![StepDraft::default()],
            ..Self::default()
        }
    }

    pub fn from_detail(detail: RecipeDetail) -> Self {
        Self {
            name: detail.recipe.name,
            source: detail.recipe.source.unwrap_or_default(),
            steps: detail
                .steps
                .into_iter()
                .map(|s| StepDraft {
                    instruction: s.instruction,
                    ingredients: s
                        .ingredients
                        .into_iter()
                        .map(|i| {
                            // Custom is legacy — present it as Count in the form
                            // so the dropdown selection round-trips.
                            let kind = match i.unit.kind() {
                                UnitKind::Custom => UnitKind::Count,
                                other => other,
                            };
                            IngDraft {
                                name: i.ingredient_name,
                                quantity: format_qty(i.quantity),
                                unit_kind: Some(kind),
                                unit: i.unit.label(),
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    fn to_payload(&self) -> Result<NewRecipe, String> {
        let mut steps = Vec::with_capacity(self.steps.len());
        for (step_idx, step) in self.steps.iter().enumerate() {
            let mut ings = Vec::with_capacity(step.ingredients.len());
            for (ing_idx, ing) in step.ingredients.iter().enumerate() {
                if ing.name.trim().is_empty() {
                    continue;
                }
                let qty_text = ing.quantity.trim();
                let quantity: f64 = if qty_text.is_empty() {
                    return Err(format!(
                        "step {}: quantity is required for `{}`",
                        step_idx + 1,
                        ing.name.trim()
                    ));
                } else {
                    qty_text.parse().map_err(|_| {
                        format!(
                            "step {} ingredient {}: `{qty_text}` is not a valid number",
                            step_idx + 1,
                            ing_idx + 1
                        )
                    })?
                };
                ings.push(NewStepIngredient {
                    ingredient_name: ing.name.clone(),
                    quantity,
                    unit_kind: ing.unit_kind,
                    unit: ing.unit.clone(),
                });
            }
            steps.push(NewStep {
                instruction: step.instruction.clone(),
                ingredients: ings,
            });
        }
        Ok(NewRecipe {
            name: self.name.clone(),
            source: {
                let t = self.source.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            },
            steps,
        })
    }
}

fn format_qty(q: f64) -> String {
    if (q.fract()).abs() < f64::EPSILON {
        format!("{}", q as i64)
    } else {
        format!("{q}")
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum RecipeFormMode {
    Create,
    Edit { id: i64 },
}

#[component]
pub fn RecipeForm(initial: RecipeDraft, mode: RecipeFormMode) -> Element {
    let mut draft = use_signal(|| initial.clone());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);
    let nav = use_navigator();

    let ingredients = use_server_future(list_ingredients)?;
    let ingredient_names: Vec<String> = match ingredients.cloned() {
        Some(Ok(list)) => list.into_iter().map(|i| i.name).collect(),
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
                RecipeFormMode::Create => create_recipe(payload).await.map_err(|e| e.to_string()),
                RecipeFormMode::Edit { id } => match update_recipe(id, payload).await {
                    Ok(()) => Ok(id),
                    Err(e) => Err(e.to_string()),
                },
            };
            match result {
                Ok(id) => {
                    nav.push(Route::RecipeDetail { id });
                }
                Err(msg) => {
                    submitting.set(false);
                    error.set(Some(msg));
                }
            }
        });
    };

    let title = match mode {
        RecipeFormMode::Create => "New recipe",
        RecipeFormMode::Edit { .. } => "Edit recipe",
    };
    let submit_label_idle = match mode {
        RecipeFormMode::Create => "Save recipe",
        RecipeFormMode::Edit { .. } => "Save changes",
    };

    rsx! {
        UnitDatalists {}
        IngredientDatalist { names: ingredient_names.clone() }

        header { class: "page-header",
            h1 { "{title}" }
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

            label {
                "Source (URL or description, optional)"
                input {
                    r#type: "text",
                    value: "{draft.read().source}",
                    oninput: move |e| draft.write().source = e.value(),
                }
            }

            h2 { "Steps" }
            {
                let step_count = draft.read().steps.len();
                let names = ingredient_names.clone();
                rsx! {
                    for step_idx in 0..step_count {
                        StepEditor {
                            key: "{step_idx}",
                            step_idx,
                            draft,
                            existing_names: names.clone(),
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                onclick: move |_| draft.write().steps.push(StepDraft::default()),
                "+ Add step"
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
                if let RecipeFormMode::Edit { id } = mode {
                    Link { to: Route::RecipeDetail { id }, class: "button-link", "Cancel" }
                }
            }
        }
    }
}

#[component]
fn UnitDatalists() -> Element {
    rsx! {
        datalist { id: MASS_LIST_ID,
            for u in MassUnit::iter() {
                option { value: "{u}" }
            }
        }
        datalist { id: VOLUME_LIST_ID,
            for u in VolumeUnit::iter() {
                option { value: "{u}" }
            }
        }
    }
}

#[component]
fn IngredientDatalist(names: Vec<String>) -> Element {
    rsx! {
        datalist { id: INGREDIENT_LIST_ID,
            for name in names {
                option { value: "{name}" }
            }
        }
    }
}

#[component]
fn StepEditor(step_idx: usize, draft: Signal<RecipeDraft>, existing_names: Vec<String>) -> Element {
    let instruction = draft
        .read()
        .steps
        .get(step_idx)
        .map(|s| s.instruction.clone())
        .unwrap_or_default();
    let ingredient_count = draft
        .read()
        .steps
        .get(step_idx)
        .map(|s| s.ingredients.len())
        .unwrap_or(0);
    let multi_step = draft.read().steps.len() > 1;

    rsx! {
        fieldset { class: "step",
            legend { "Step {step_idx + 1}" }

            label {
                "Instruction"
                textarea {
                    rows: "3",
                    value: "{instruction}",
                    oninput: move |e| {
                        let mut d = draft.write();
                        if let Some(step) = d.steps.get_mut(step_idx) {
                            step.instruction = e.value();
                        }
                    },
                }
            }

            div { class: "ingredients-editor",
                h3 { "Ingredients" }
                for ing_idx in 0..ingredient_count {
                    IngredientEditor {
                        key: "{step_idx}-{ing_idx}",
                        step_idx,
                        ing_idx,
                        draft,
                        existing_names: existing_names.clone(),
                    }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    onclick: move |_| {
                        let mut d = draft.write();
                        if let Some(step) = d.steps.get_mut(step_idx) {
                            step.ingredients.push(IngDraft::default());
                        }
                    },
                    "+ Add ingredient"
                }
            }

            if multi_step {
                button {
                    r#type: "button",
                    class: "danger",
                    onclick: move |_| {
                        draft.write().steps.remove(step_idx);
                    },
                    "Remove step"
                }
            }
        }
    }
}

#[component]
fn IngredientEditor(
    step_idx: usize,
    ing_idx: usize,
    draft: Signal<RecipeDraft>,
    existing_names: Vec<String>,
) -> Element {
    let row = draft
        .read()
        .steps
        .get(step_idx)
        .and_then(|s| s.ingredients.get(ing_idx).cloned())
        .unwrap_or_default();
    let kind = row.unit_kind;
    let status = ingredient_status(&row.name, &existing_names);

    rsx! {
        div { class: "ingredient-row",
            input {
                r#type: "text",
                inputmode: "decimal",
                class: "qty",
                placeholder: "qty",
                value: "{row.quantity}",
                oninput: move |e| {
                    let mut d = draft.write();
                    if let Some(ing) = d
                        .steps
                        .get_mut(step_idx)
                        .and_then(|s| s.ingredients.get_mut(ing_idx))
                    {
                        ing.quantity = e.value();
                    }
                },
            }
            select {
                value: "{unit_kind_value(kind)}",
                oninput: move |e| {
                    let v = e.value();
                    let mut d = draft.write();
                    if let Some(ing) = d
                        .steps
                        .get_mut(step_idx)
                        .and_then(|s| s.ingredients.get_mut(ing_idx))
                    {
                        ing.unit_kind = UnitKind::from_str(&v).ok();
                        ing.unit.clear();
                    }
                },
                option { value: "", "—" }
                option { value: "mass", "mass" }
                option { value: "volume", "volume" }
                option { value: "count", "count" }
            }
            input {
                r#type: "text",
                placeholder: "unit",
                value: "{row.unit}",
                list: unit_list_for(kind),
                oninput: move |e| {
                    let mut d = draft.write();
                    if let Some(ing) = d
                        .steps
                        .get_mut(step_idx)
                        .and_then(|s| s.ingredients.get_mut(ing_idx))
                    {
                        ing.unit = e.value();
                    }
                },
            }
            div { class: "name-cell",
                input {
                    r#type: "text",
                    placeholder: "name",
                    value: "{row.name}",
                    list: INGREDIENT_LIST_ID,
                    oninput: move |e| {
                        let mut d = draft.write();
                        if let Some(ing) = d
                            .steps
                            .get_mut(step_idx)
                            .and_then(|s| s.ingredients.get_mut(ing_idx))
                        {
                            ing.name = e.value();
                        }
                    },
                }
                match status {
                    IngredientStatus::Empty => rsx! {},
                    IngredientStatus::Existing => rsx! {
                        span { class: "ingredient-status existing", "✓ existing" }
                    },
                    IngredientStatus::New => rsx! {
                        span { class: "ingredient-status new", "✨ new ingredient" }
                    },
                }
            }
            button {
                r#type: "button",
                class: "danger small",
                onclick: move |_| {
                    let mut d = draft.write();
                    if let Some(step) = d.steps.get_mut(step_idx)
                        && ing_idx < step.ingredients.len()
                    {
                        step.ingredients.remove(ing_idx);
                    }
                },
                "×"
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum IngredientStatus {
    Empty,
    Existing,
    New,
}

fn ingredient_status(name: &str, existing: &[String]) -> IngredientStatus {
    let t = name.trim();
    if t.is_empty() {
        IngredientStatus::Empty
    } else if existing.iter().any(|n| n.eq_ignore_ascii_case(t)) {
        IngredientStatus::Existing
    } else {
        IngredientStatus::New
    }
}

fn unit_kind_value(uk: Option<UnitKind>) -> String {
    uk.map(|k| k.to_string()).unwrap_or_default()
}

fn unit_list_for(uk: Option<UnitKind>) -> Option<&'static str> {
    match uk {
        Some(UnitKind::Mass) => Some(MASS_LIST_ID),
        Some(UnitKind::Volume) => Some(VOLUME_LIST_ID),
        _ => None,
    }
}
