use crate::{Route, draft_id::DraftId};
use api::{create_recipe, list_ingredients, update_recipe};
use dioxus::prelude::*;
use std::str::FromStr;
use strum::IntoEnumIterator;
use types::{Mass, NewRecipe, NewStep, NewStepIngredient, RecipeDetail, UnitKind, Volume};
use ui::TrashIcon;

#[derive(Default, Clone, PartialEq)]
pub struct IngDraft {
    pub id: DraftId,
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

impl IngDraft {
    fn key(&self) -> String {
        format!("ingredient-{}", self.id)
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct StepDraft {
    pub id: DraftId,
    pub instruction: String,
    pub ingredients: Vec<IngDraft>,
    /// Counter for allocating ids to ingredients added inside this step.
    /// Per-step so server/client SSR produce matching ids on first render.
    next_ing_id: i64,
}

impl StepDraft {
    fn key(&self) -> String {
        format!("step-{}", self.id)
    }

    fn alloc_ing_id(&mut self) -> DraftId {
        let id = DraftId::New(self.next_ing_id);
        self.next_ing_id += 1;
        id
    }

    fn push_new_ingredient(&mut self) -> DraftId {
        let id = self.alloc_ing_id();
        self.ingredients.push(IngDraft {
            id,
            ..Default::default()
        });
        id
    }
}

#[derive(Default, Clone, PartialEq)]
pub struct RecipeDraft {
    pub id: DraftId,
    pub name: String,
    pub source: String,
    pub steps: Vec<StepDraft>,
    /// Counter for allocating ids to steps added to this draft.
    next_step_id: i64,
}

impl RecipeDraft {
    fn alloc_step_id(&mut self) -> DraftId {
        let id = DraftId::New(self.next_step_id);
        self.next_step_id += 1;
        id
    }

    fn push_new_step(&mut self) -> DraftId {
        let id = self.alloc_step_id();
        self.steps.push(StepDraft {
            id,
            ..Default::default()
        });
        id
    }

    pub fn empty() -> Self {
        let mut d = Self::default();
        d.push_new_step();
        d
    }

    pub fn from_detail(detail: RecipeDetail) -> Self {
        Self {
            id: detail.recipe.id.into(),
            name: detail.recipe.name,
            source: detail.recipe.source.unwrap_or_default(),
            steps: detail
                .steps
                .into_iter()
                .map(|s| StepDraft {
                    id: s.id.into(),
                    instruction: s.instruction,
                    ingredients: s
                        .ingredients
                        .into_iter()
                        .map(|i| IngDraft {
                            id: i.id.into(),
                            name: i.ingredient_name,
                            quantity: format_qty(i.quantity),
                            unit: i.unit.label(),
                        })
                        .collect(),
                    next_ing_id: 0,
                })
                .collect(),
            next_step_id: 0,
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
    } else if Mass::from_str(t).is_ok() {
        UnitKind::Mass
    } else if Volume::from_str(t).is_ok() {
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
        Some(Err(e)) => {
            return rsx! {
                p { class: "error", "Failed to load ingredients: {e}" }
            };
        }
        None => Vec::new(),
    };

    let unit_options: Vec<String> = Mass::iter()
        .map(|u| u.to_string())
        .chain(Volume::iter().map(|u| u.to_string()))
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

    let mut add_step_fn = move || {
        let id = draft.write().push_new_step();
        focus_field(format!("instr-step-{id}"));
    };
    let add_step = Callback::new(move |_| add_step_fn());

    let on_form_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter && has_command_modifier(&e.modifiers()) {
            e.prevent_default();
            add_step.call(());
        }
    };

    let steps_snapshot: Vec<(DraftId, String, usize)> = draft
        .read()
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, s.key(), i))
        .collect();
    let multi_step = steps_snapshot.len() > 1;

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
                            if let Some(first) = draft.read().steps.first() {
                                focus_field(format!("instr-{}", first.key()));
                            }
                        }
                    },
                }
            }

            h2 { "Steps" }
            for (step_id, step_key, step_idx) in steps_snapshot.into_iter() {
                StepEditor {
                    key: "{step_key}",
                    draft,
                    step_id,
                    step_idx,
                    multi_step,
                    existing_names: ingredient_names.clone(),
                    unit_options: unit_options.clone(),
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                onclick: move |_| add_step.call(()),
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

/// Find a step by id and run the closure with a mutable reference to it.
/// No-op if the step has been removed.
fn with_step(draft: &mut Signal<RecipeDraft>, step_id: DraftId, f: impl FnOnce(&mut StepDraft)) {
    let mut d = draft.write();
    if let Some(s) = d.steps.iter_mut().find(|s| s.id == step_id) {
        f(s);
    }
}

/// Find an ingredient by ids and run the closure with a mutable reference.
/// No-op if either has been removed.
fn with_ingredient(
    draft: &mut Signal<RecipeDraft>,
    step_id: DraftId,
    ing_id: DraftId,
    f: impl FnOnce(&mut IngDraft),
) {
    with_step(draft, step_id, |s| {
        if let Some(i) = s.ingredients.iter_mut().find(|i| i.id == ing_id) {
            f(i);
        }
    });
}

#[component]
fn StepEditor(
    draft: Signal<RecipeDraft>,
    step_id: DraftId,
    step_idx: usize,
    multi_step: bool,
    existing_names: Vec<String>,
    unit_options: Vec<String>,
) -> Element {
    let mut draft = draft;
    let Some(step_snapshot) = draft.read().steps.iter().find(|s| s.id == step_id).cloned() else {
        return rsx! {};
    };

    let step_key = step_snapshot.key();
    let ingredients_snapshot: Vec<(DraftId, String)> = step_snapshot
        .ingredients
        .iter()
        .map(|i| (i.id, i.key()))
        .collect();

    let mut add_ingredient_fn = move || {
        let mut new_id = None;
        with_step(&mut draft, step_id, |s| {
            new_id = Some(s.push_new_ingredient());
        });
        if let Some(id) = new_id {
            focus_field(format!("qty-step-{step_id}-ingredient-{id}"));
        }
    };
    let add_ingredient = Callback::new(move |_| add_ingredient_fn());

    rsx! {
        fieldset { class: "step",
            legend { "Step {step_idx + 1}" }

            div { class: "ingredients-editor",
                h3 { "Ingredients" }
                for (ing_id, ing_key) in ingredients_snapshot.into_iter() {
                    IngredientEditor {
                        key: "{ing_key}",
                        draft,
                        step_id,
                        ing_id,
                        focus_key_suffix: format!("{}-{}", step_key, ing_key),
                        on_enter_add: add_ingredient,
                        existing_names: existing_names.clone(),
                        unit_options: unit_options.clone(),
                    }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    "data-focus-key": "instr-{step_key}",
                    onclick: move |_| add_ingredient.call(()),
                    "+ Add ingredient"
                }
            }

            label {
                "Instruction"
                // NOTE: do not add `initial_value:` here. There's a Dioxus 0.7
                // hydration/diff bug where a textarea with `initial_value`
                // breaks the parent's VDOM diff when a sibling list grows
                // (e.g. clicking "+ Add step"), producing a null-DOM-node crash
                // in the interpreter. The body text below is what the textarea
                // shows on first render and after re-renders.
                textarea {
                    rows: "1",
                    onmounted: {
                        let step_key = step_key.clone();
                        move |_| autogrow_textarea(format!("instr-{step_key}"))
                    },
                    oninput: {
                        let step_key = step_key.clone();
                        move |e: FormEvent| {
                            let value = e.value();
                            with_step(&mut draft, step_id, |s| s.instruction = value);
                            autogrow_textarea(format!("instr-{step_key}"));
                        }
                    },
                    "{step_snapshot.instruction}"
                }
            }

            if multi_step {
                button {
                    r#type: "button",
                    class: "icon-button",
                    tabindex: "-1",
                    "aria-label": "Remove step",
                    title: "Remove step",
                    onclick: move |_| {
                        draft.write().steps.retain(|s| s.id != step_id);
                    },
                    TrashIcon {}
                }
            }
        }
    }
}

#[component]
fn IngredientEditor(
    draft: Signal<RecipeDraft>,
    step_id: DraftId,
    ing_id: DraftId,
    focus_key_suffix: String,
    on_enter_add: Callback<()>,
    existing_names: Vec<String>,
    unit_options: Vec<String>,
) -> Element {
    let mut draft = draft;
    let Some(row) = draft
        .read()
        .steps
        .iter()
        .find(|s| s.id == step_id)
        .and_then(|s| s.ingredients.iter().find(|i| i.id == ing_id))
        .cloned()
    else {
        return rsx! {};
    };
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
                oninput: move |e: FormEvent| {
                    let v = e.value();
                    with_ingredient(&mut draft, step_id, ing_id, |i| i.quantity = v);
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
                    with_ingredient(&mut draft, step_id, ing_id, |i| i.unit = v);
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
                        with_ingredient(&mut draft, step_id, ing_id, |i| i.name = v);
                    },
                    onenter: move |_| {
                        let name_empty = draft
                            .read()
                            .steps
                            .iter()
                            .find(|s| s.id == step_id)
                            .and_then(|s| s.ingredients.iter().find(|i| i.id == ing_id))
                            .map(|i| i.name.trim().is_empty())
                            .unwrap_or(true);
                        if name_empty {
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
                class: "icon-button",
                tabindex: "-1",
                "aria-label": "Remove ingredient",
                title: "Remove ingredient",
                onclick: move |_| {
                    with_step(&mut draft, step_id, |s| {
                        s.ingredients.retain(|i| i.id != ing_id);
                    });
                },
                TrashIcon {}
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
                            key: "{name}",
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
