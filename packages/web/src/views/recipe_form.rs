use crate::Route;
use api::{create_recipe, list_ingredients, update_recipe};
use dioxus::prelude::*;
use std::str::FromStr;
use strum::IntoEnumIterator;
use types::{MassUnit, NewRecipe, NewStep, NewStepIngredient, RecipeDetail, UnitKind, VolumeUnit};

#[derive(Default, Clone, PartialEq)]
pub struct IngDraft {
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

#[derive(Default, Clone, PartialEq)]
pub struct StepDraft {
    pub instruction: String,
    pub ingredients: Vec<Signal<IngDraft>>,
}

#[derive(Default, Clone, PartialEq)]
pub struct RecipeDraft {
    pub name: String,
    pub source: String,
    pub steps: Vec<Signal<StepDraft>>,
}

impl RecipeDraft {
    pub fn empty() -> Self {
        Self {
            steps: vec![Signal::new(StepDraft::default())],
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
                .map(|s| {
                    Signal::new(StepDraft {
                        instruction: s.instruction,
                        ingredients: s
                            .ingredients
                            .into_iter()
                            .map(|i| {
                                Signal::new(IngDraft {
                                    name: i.ingredient_name,
                                    quantity: format_qty(i.quantity),
                                    unit: i.unit.label(),
                                })
                            })
                            .collect(),
                    })
                })
                .collect(),
        }
    }

    fn to_payload(&self) -> Result<NewRecipe, String> {
        let mut steps = Vec::with_capacity(self.steps.len());
        for (step_idx, step_sig) in self.steps.iter().enumerate() {
            let step = step_sig.read();
            let mut ings = Vec::with_capacity(step.ingredients.len());
            for (ing_idx, ing_sig) in step.ingredients.iter().enumerate() {
                let ing = ing_sig.read();
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
                    unit_kind: Some(derive_unit_kind(&ing.unit)),
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

fn derive_unit_kind(text: &str) -> UnitKind {
    let t = text.trim();
    if t.is_empty() {
        UnitKind::Count
    } else if MassUnit::from_str(t).is_ok() {
        UnitKind::Mass
    } else if VolumeUnit::from_str(t).is_ok() {
        UnitKind::Volume
    } else {
        UnitKind::Count
    }
}

fn format_qty(q: f64) -> String {
    if (q.fract()).abs() < f64::EPSILON {
        format!("{}", q as i64)
    } else {
        format!("{q}")
    }
}

/// Focus the element with the matching `data-focus-key`. Deferred via
/// `requestAnimationFrame` so it works for elements added in the same tick.
fn focus_field(key: String) {
    spawn(async move {
        let safe = key.replace('"', "");
        let _ = document::eval(&format!(
            "requestAnimationFrame(() => {{ const el = document.querySelector('[data-focus-key=\"{safe}\"]'); if (el) el.focus(); }})"
        ))
        .await;
    });
}

// Firefox <152 doesn't support CSS `field-sizing: content`, so size the textarea
// from JS. Once Firefox 152+ is widespread, the CSS rule alone is sufficient and
// this can be removed.
fn autogrow_textarea(key: String) {
    spawn(async move {
        let safe = key.replace('"', "");
        let _ = document::eval(&format!(
            "requestAnimationFrame(() => {{ const el = document.querySelector('[data-focus-key=\"{safe}\"]'); if (el) {{ el.style.height = 'auto'; el.style.height = el.scrollHeight + 'px'; }} }})"
        ))
        .await;
    });
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

    let unit_options: Vec<String> = MassUnit::iter()
        .map(|u| u.to_string())
        .chain(VolumeUnit::iter().map(|u| u.to_string()))
        .collect();

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

    let on_form_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter && has_command_modifier(&e.modifiers()) {
            e.prevent_default();
            let new_step_idx = {
                let mut d = draft.write();
                let i = d.steps.len();
                d.steps.push(Signal::new(StepDraft::default()));
                i
            };
            focus_field(format!("instr-{new_step_idx}"));
        }
    };

    rsx! {
        header { class: "page-header",
            h1 { "{title}" }
        }

        form {
            class: "recipe-form",
            onsubmit: submit,
            onkeydown: on_form_keydown,

            label {
                "Name"
                input {
                    r#type: "text",
                    required: true,
                    value: "{draft.read().name}",
                    "data-focus-key": "recipe-name",
                    oninput: move |e| draft.write().name = e.value(),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && e.modifiers().is_empty() {
                            e.prevent_default();
                            focus_field("recipe-source".to_string());
                        }
                    },
                }
            }

            label {
                "Source (URL or description, optional)"
                input {
                    r#type: "text",
                    value: "{draft.read().source}",
                    "data-focus-key": "recipe-source",
                    oninput: move |e| draft.write().source = e.value(),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && e.modifiers().is_empty() {
                            e.prevent_default();
                            focus_field("instr-0".to_string());
                        }
                    },
                }
            }

            h2 { "Steps" }
            {
                let steps = draft.read().steps.clone();
                let multi_step = steps.len() > 1;
                let names = ingredient_names.clone();
                let units = unit_options.clone();
                rsx! {
                    for (step_idx, step) in steps.into_iter().enumerate() {
                        StepEditor {
                            key: "{step_idx}",
                            step,
                            step_idx,
                            multi_step,
                            on_remove: Callback::new(move |_| {
                                draft.write().steps.remove(step_idx);
                            }),
                            existing_names: names.clone(),
                            unit_options: units.clone(),
                        }
                    }
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                onclick: move |_| {
                    let new_step_idx = {
                        let mut d = draft.write();
                        let i = d.steps.len();
                        d.steps.push(Signal::new(StepDraft::default()));
                        i
                    };
                    focus_field(format!("instr-{new_step_idx}"));
                },
                "+ Add step "
                span { class: "kbd-hint", "(Ctrl+Enter)" }
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

fn has_command_modifier(m: &Modifiers) -> bool {
    m.contains(Modifiers::CONTROL) || m.contains(Modifiers::META)
}

#[component]
fn StepEditor(
    mut step: Signal<StepDraft>,
    step_idx: usize,
    multi_step: bool,
    on_remove: Callback<()>,
    existing_names: Vec<String>,
    unit_options: Vec<String>,
) -> Element {
    let instruction = step.read().instruction.clone();
    let ingredients = step.read().ingredients.clone();

    rsx! {
        fieldset { class: "step",
            legend { "Step {step_idx + 1}" }

            label {
                "Instruction"
                textarea {
                    rows: "1",
                    "data-focus-key": "instr-{step_idx}",
                    onmounted: move |_| autogrow_textarea(format!("instr-{step_idx}")),
                    oninput: move |e| {
                        step.write().instruction = e.value();
                        autogrow_textarea(format!("instr-{step_idx}"));
                    },
                    initial_value: "{instruction}",
                    "{instruction}"
                }
            }

            div { class: "ingredients-editor",
                h3 { "Ingredients" }
                for (ing_idx, ing) in ingredients.into_iter().enumerate() {
                    IngredientEditor {
                        key: "{step_idx}-{ing_idx}",
                        ing,
                        focus_key_suffix: format!("{step_idx}-{ing_idx}"),
                        on_remove: Callback::new(move |_| {
                            let mut s = step.write();
                            if ing_idx < s.ingredients.len() {
                                s.ingredients.remove(ing_idx);
                            }
                        }),
                        on_enter_add: Callback::new(move |_| {
                            let new_idx = {
                                let mut s = step.write();
                                let i = s.ingredients.len();
                                s.ingredients.push(Signal::new(IngDraft::default()));
                                i
                            };
                            focus_field(format!("qty-{step_idx}-{new_idx}"));
                        }),
                        existing_names: existing_names.clone(),
                        unit_options: unit_options.clone(),
                    }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    onclick: move |_| {
                        let new_idx = {
                            let mut s = step.write();
                            let i = s.ingredients.len();
                            s.ingredients.push(Signal::new(IngDraft::default()));
                            i
                        };
                        focus_field(format!("qty-{step_idx}-{new_idx}"));
                    },
                    "+ Add ingredient"
                }
            }

            if multi_step {
                button {
                    r#type: "button",
                    class: "danger",
                    tabindex: "-1",
                    onclick: move |_| on_remove.call(()),
                    "Remove step"
                }
            }
        }
    }
}

#[component]
fn IngredientEditor(
    mut ing: Signal<IngDraft>,
    focus_key_suffix: String,
    on_remove: Callback<()>,
    on_enter_add: Callback<()>,
    existing_names: Vec<String>,
    unit_options: Vec<String>,
) -> Element {
    let row = ing.read().clone();
    let status = ingredient_status(&row.name, &existing_names);

    let qty_focus_key = format!("qty-{focus_key_suffix}");
    let unit_focus_key = format!("unit-{focus_key_suffix}");
    let name_focus_key = format!("name-{focus_key_suffix}");

    let unit_key_for_qty_enter = unit_focus_key.clone();
    let name_key_for_unit_enter = name_focus_key.clone();

    rsx! {
        div { class: "ingredient-row",
            input {
                r#type: "text",
                inputmode: "decimal",
                class: "qty",
                placeholder: "qty",
                value: "{row.quantity}",
                autocomplete: "off",
                "data-focus-key": "{qty_focus_key}",
                oninput: move |e| {
                    ing.write().quantity = e.value();
                },
                onkeydown: move |e| {
                    if e.key() == Key::Enter && e.modifiers().is_empty() {
                        e.prevent_default();
                        focus_field(unit_key_for_qty_enter.clone());
                    }
                },
            }
            Autocomplete {
                value: row.unit.clone(),
                options: unit_options.clone(),
                placeholder: "unit".to_string(),
                focus_key: unit_focus_key.clone(),
                wrapper_class: "unit-cell".to_string(),
                oninput: move |v: String| {
                    ing.write().unit = v;
                },
                onenter: move |_| {
                    focus_field(name_key_for_unit_enter.clone());
                },
            }
            div { class: "name-cell",
                Autocomplete {
                    value: row.name.clone(),
                    options: existing_names.clone(),
                    placeholder: "name".to_string(),
                    focus_key: name_focus_key.clone(),
                    wrapper_class: String::new(),
                    oninput: move |v: String| {
                        ing.write().name = v;
                    },
                    onenter: move |_| {
                        if ing.read().name.trim().is_empty() {
                            return;
                        }
                        on_enter_add.call(());
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
                tabindex: "-1",
                onclick: move |_| on_remove.call(()),
                "×"
            }
        }
    }
}

#[component]
fn Autocomplete(
    value: String,
    options: Vec<String>,
    placeholder: String,
    focus_key: String,
    wrapper_class: String,
    oninput: EventHandler<String>,
    onenter: EventHandler<()>,
) -> Element {
    // `open` is true only when the user is actively browsing suggestions
    // (typing or ArrowDown). Focus alone doesn't open it, so re-entering an
    // already-filled field doesn't surprise the user with a popup.
    let mut open = use_signal(|| false);
    // `highlight_offset` lets the user move the selection relative to the
    // first match. `None` means "first match" (the default). ArrowUp past the
    // top dismisses the popup, so there's no "no selection" state while open.
    let mut highlight_offset = use_signal(|| 0usize);

    let filtered = filter_options(&options, &value);
    let popup_visible = open() && !filtered.is_empty();
    let highlight_idx = if popup_visible {
        highlight_offset().min(filtered.len() - 1)
    } else {
        0
    };
    let filtered_for_keys = filtered.clone();

    rsx! {
        div { class: "autocomplete {wrapper_class}",
            input {
                r#type: "text",
                placeholder: "{placeholder}",
                value: "{value}",
                autocomplete: "off",
                "data-focus-key": "{focus_key}",
                oninput: move |e| {
                    let v = e.value();
                    open.set(!v.trim().is_empty());
                    highlight_offset.set(0);
                    oninput.call(v);
                },
                onblur: move |_| {
                    open.set(false);
                    highlight_offset.set(0);
                },
                onkeydown: {
                    let filtered = filtered_for_keys.clone();
                    move |e: KeyboardEvent| {
                        let n = filtered.len();
                        match e.key() {
                            Key::ArrowDown => {
                                e.prevent_default();
                                if n > 0 {
                                    if !open() {
                                        open.set(true);
                                        highlight_offset.set(0);
                                    } else {
                                        let cur = highlight_offset().min(n - 1);
                                        highlight_offset.set((cur + 1) % n);
                                    }
                                }
                            }
                            Key::ArrowUp
                                if open() && n > 0 => {
                                e.prevent_default();
                                let cur = highlight_offset().min(n - 1);
                                if cur == 0 {
                                    // moving up off the top dismisses
                                    open.set(false);
                                    highlight_offset.set(0);
                                } else {
                                    highlight_offset.set(cur - 1);
                                }
                            }
                            Key::Escape
                                if open() => {
                                e.prevent_default();
                                open.set(false);
                                highlight_offset.set(0);
                            }
                            Key::Tab
                                // Accept selection if popup is open, then let
                                // Tab keep its default focus-advance behavior.
                                if open() && n > 0 => {
                                let i = highlight_offset().min(n - 1);
                                if let Some(s) = filtered.get(i) {
                                    oninput.call(s.clone());
                                }
                                open.set(false);
                                highlight_offset.set(0);
                            }
                            Key::Enter => {
                                if has_command_modifier(&e.modifiers()) {
                                    return;
                                }
                                e.prevent_default();
                                if open() && n > 0 {
                                    let i = highlight_offset().min(n - 1);
                                    if let Some(s) = filtered.get(i) {
                                        oninput.call(s.clone());
                                    }
                                    open.set(false);
                                    highlight_offset.set(0);
                                }
                                onenter.call(());
                            }
                            _ => {}
                        }
                    }
                },
            }
            if popup_visible {
                ul { class: "autocomplete-popup",
                    for (i, name) in filtered.iter().enumerate() {
                        AutocompleteItem {
                            key: "{i}",
                            index: i,
                            text: name.clone(),
                            active: i == highlight_idx,
                            oninput,
                            open,
                            highlight_offset,
                        }
                    }
                }
            }
        }
    }
}

fn filter_options(options: &[String], value: &str) -> Vec<String> {
    let needle = value.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &String)> = options
        .iter()
        .filter_map(|o| {
            let lo = o.to_lowercase();
            if lo == needle {
                None
            } else {
                lo.find(&needle).map(|p| (p, o))
            }
        })
        .collect();
    scored.sort_by_key(|(p, _)| *p);
    scored.into_iter().map(|(_, o)| o.clone()).take(8).collect()
}

#[component]
fn AutocompleteItem(
    index: usize,
    text: String,
    active: bool,
    oninput: EventHandler<String>,
    open: Signal<bool>,
    highlight_offset: Signal<usize>,
) -> Element {
    let class = if active { "active" } else { "" };
    let text_for_click = text.clone();
    let mut open = open;
    let mut highlight_offset = highlight_offset;
    rsx! {
        li {
            class: "{class}",
            onmousedown: move |e| {
                e.prevent_default();
                oninput.call(text_for_click.clone());
                open.set(false);
                highlight_offset.set(0);
            },
            onmouseenter: move |_| {
                highlight_offset.set(index);
            },
            "{text}"
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
