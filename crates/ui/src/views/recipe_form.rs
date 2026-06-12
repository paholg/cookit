use {
    crate::{
        ClientOnly, Route,
        client::client,
        icons::{InsertAboveIcon, TrashIcon},
        use_confirm,
    },
    api::{
        RecipeBuilder, RecipeStepBuilder, RecipeStepIngredientBuilder, delete_recipe,
        duration::{format_duration, parse_duration},
        id::{DraftId, RecipeStepDraftId, RecipeStepIngredientDraftId},
        list_ingredients,
        unit::{Mass, Volume},
        upsert_recipe,
    },
    dioxus::prelude::*,
    std::collections::HashMap,
    strum::IntoEnumIterator,
};

fn step_key(id: RecipeStepDraftId) -> String {
    format!("step-{id}")
}

fn ing_key(id: RecipeStepIngredientDraftId) -> String {
    format!("ingredient-{id}")
}

/// Append an empty step, returning its freshly allocated id.
fn push_new_step(draft: &mut Signal<RecipeBuilder>) -> RecipeStepDraftId {
    let mut d = draft.write();
    let id = DraftId::next(d.steps.iter().map(|s| s.id));
    d.steps.push(RecipeStepBuilder {
        id,
        ..Default::default()
    });
    id
}

/// Insert an empty step at `idx`, returning its freshly allocated id.
fn insert_new_step_at(draft: &mut Signal<RecipeBuilder>, idx: usize) -> RecipeStepDraftId {
    let mut d = draft.write();
    let id = DraftId::next(d.steps.iter().map(|s| s.id));
    d.steps.insert(
        idx,
        RecipeStepBuilder {
            id,
            ..Default::default()
        },
    );
    id
}

/// Append an empty ingredient row to a step, returning its id. `None` if the
/// step has been removed.
fn push_new_ingredient(
    draft: &mut Signal<RecipeBuilder>,
    step_id: RecipeStepDraftId,
) -> Option<RecipeStepIngredientDraftId> {
    let mut d = draft.write();
    let step = d.steps.iter_mut().find(|s| s.id == step_id)?;
    let id = DraftId::next(step.ingredients.iter().map(|i| i.id));
    step.ingredients.push(RecipeStepIngredientBuilder {
        id,
        ..Default::default()
    });
    Some(id)
}

/// Focus the element with the matching `data-focus-key`.
fn focus_field(key: String) {
    client().focus_field(&key);
}

/// Resize the matching `data-autogrow` textarea to fit its content.
fn autogrow_textarea(key: String) {
    client().autogrow_textarea(&key);
}

#[derive(Clone, PartialEq)]
pub enum RecipeFormMode {
    Create,
    Edit { recipe_key: String },
}

#[component]
pub fn RecipeForm(initial: RecipeBuilder, mode: RecipeFormMode) -> Element {
    let mut draft = use_signal(|| initial.clone());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let confirm = use_confirm();
    // Transient per-step timer parse errors, surfaced on blur. Not part of the
    // wire payload, so they live alongside the draft rather than inside it.
    let duration_errors = use_signal(HashMap::<RecipeStepDraftId, String>::new);
    let nav = use_navigator();

    let ingredients = use_server_future(list_ingredients)?;
    let ingredient_names: Vec<String> = match ingredients.cloned() {
        Some(Ok(list)) => list
            .into_iter()
            .map(|i| i.name.as_ref().to_string())
            .collect(),
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

        let payload = draft.read().clone();
        if let Err(err) = payload.validate() {
            error.set(Some(err.summary()));
            return;
        }

        submitting.set(true);
        error.set(None);

        spawn(async move {
            match upsert_recipe(payload).await {
                Ok(detail) => {
                    nav.push(Route::RecipeDetail {
                        recipe_key: detail.recipe.slug.to_string(),
                    });
                }
                Err(e) => {
                    submitting.set(false);
                    error.set(Some(e.to_string()));
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
        let id = push_new_step(&mut draft);
        focus_field(format!("instr-step-{id}"));
    };
    let add_step = Callback::new(move |_| add_step_fn());

    let on_form_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter && has_command_modifier(&e.modifiers()) {
            e.prevent_default();
            add_step.call(());
        }
    };

    let steps_snapshot: Vec<(RecipeStepDraftId, String, usize)> = draft
        .read()
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, step_key(s.id), i))
        .collect();
    let multi_step = steps_snapshot.len() > 1;

    rsx! {
        header { class: "page-header",
            h1 { "{title}" }
            if let RecipeFormMode::Edit { recipe_key } = mode.clone() {
                button {
                    r#type: "button",
                    class: "icon-button trash",
                    "aria-label": "Delete recipe",
                    title: "Delete recipe",
                    disabled: deleting() || submitting(),
                    onclick: move |_| {
                        if deleting() { return; }
                        let recipe_key = recipe_key.clone();
                        spawn(async move {
                            let confirmed = confirm
                                .show("Delete this recipe? This cannot be undone.")
                                .await;
                            if !confirmed { return; }

                            deleting.set(true);
                            error.set(None);
                            match delete_recipe(recipe_key).await {
                                Ok(()) => {
                                    nav.push(Route::RecipeList {});
                                }
                                Err(e) => {
                                    error.set(Some(e.to_string()));
                                    deleting.set(false);
                                }
                            }
                        });
                    },
                    TrashIcon {}
                }
            }
        }

        form {
            class: "app-form",
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
                                focus_field(format!("instr-{}", step_key(first.id)));
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
                    duration_errors,
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
                if let RecipeFormMode::Edit { recipe_key } = mode.clone() {
                    Link { to: Route::RecipeDetail { recipe_key }, class: "button secondary", "Cancel" }
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

fn has_command_modifier(m: &Modifiers) -> bool {
    m.contains(Modifiers::CONTROL) || m.contains(Modifiers::META)
}

/// Find a step by id and run the closure with a mutable reference to it.
/// No-op if the step has been removed.
fn with_step(
    draft: &mut Signal<RecipeBuilder>,
    step_id: RecipeStepDraftId,
    f: impl FnOnce(&mut RecipeStepBuilder),
) {
    let mut d = draft.write();
    if let Some(s) = d.steps.iter_mut().find(|s| s.id == step_id) {
        f(s);
    }
}

/// Find an ingredient by ids and run the closure with a mutable reference.
/// No-op if either has been removed.
fn with_ingredient(
    draft: &mut Signal<RecipeBuilder>,
    step_id: RecipeStepDraftId,
    ing_id: RecipeStepIngredientDraftId,
    f: impl FnOnce(&mut RecipeStepIngredientBuilder),
) {
    with_step(draft, step_id, |s| {
        if let Some(i) = s.ingredients.iter_mut().find(|i| i.id == ing_id) {
            f(i);
        }
    });
}

#[component]
fn StepEditor(
    draft: Signal<RecipeBuilder>,
    duration_errors: Signal<HashMap<RecipeStepDraftId, String>>,
    step_id: RecipeStepDraftId,
    step_idx: usize,
    multi_step: bool,
    existing_names: Vec<String>,
    unit_options: Vec<String>,
) -> Element {
    let mut draft = draft;
    let mut duration_errors = duration_errors;
    let Some(step_snapshot) = draft.read().steps.iter().find(|s| s.id == step_id).cloned() else {
        return rsx! {};
    };

    let step_key = step_key(step_id);
    let duration_error = duration_errors.read().get(&step_id).cloned();
    let ingredients_snapshot: Vec<(RecipeStepIngredientDraftId, String)> = step_snapshot
        .ingredients
        .iter()
        .map(|i| (i.id, ing_key(i.id)))
        .collect();

    let mut add_ingredient_fn = move || {
        if let Some(id) = push_new_ingredient(&mut draft, step_id) {
            focus_field(format!("qty-step-{step_id}-ingredient-{id}"));
        }
    };
    let add_ingredient = Callback::new(move |_| add_ingredient_fn());

    rsx! {
        fieldset { class: "step",
            legend { class: "step-legend",
                span { class: "step-title", "Step {step_idx + 1}" }
                span { class: "step-actions",
                    button {
                        r#type: "button",
                        class: "icon-button",
                        tabindex: "-1",
                        "aria-label": "Insert step above",
                        title: "Insert step above",
                        onclick: move |_| {
                            let id = insert_new_step_at(&mut draft, step_idx);
                            focus_field(format!("instr-step-{id}"));
                        },
                        InsertAboveIcon {}
                    }
                    if multi_step {
                        button {
                            r#type: "button",
                            class: "icon-button trash",
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
                ClientOnly {
                    textarea {
                        rows: "1",
                        value: "{step_snapshot.instruction}",
                        "data-autogrow": "instr-{step_key}",
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
                    }
                }
            }

            label { class: "step-duration",
                "Timer (optional)"
                input {
                    r#type: "text",
                    class: if duration_error.is_some() { "duration-input invalid" } else { "duration-input" },
                    placeholder: "e.g. 30s, 1h 30m",
                    value: "{step_snapshot.duration_text}",
                    oninput: move |e: FormEvent| {
                        let v = e.value();
                        with_step(&mut draft, step_id, |s| s.duration_text = v);
                        // Clear stale error as soon as the user edits.
                        duration_errors.write().remove(&step_id);
                    },
                    onblur: move |_| {
                        let mut err = None;
                        with_step(&mut draft, step_id, |s| {
                            let trimmed = s.duration_text.trim();
                            if trimmed.is_empty() {
                                s.duration_text.clear();
                            } else {
                                match parse_duration(trimmed) {
                                    Ok(secs) => s.duration_text = format_duration(secs),
                                    Err(msg) => err = Some(msg),
                                }
                            }
                        });
                        match err {
                            Some(msg) => { duration_errors.write().insert(step_id, msg); }
                            None => { duration_errors.write().remove(&step_id); }
                        }
                    },
                }
                if let Some(err) = duration_error.as_ref() {
                    span { class: "duration-error", "{err}" }
                }
            }
        }
    }
}

#[component]
fn IngredientEditor(
    draft: Signal<RecipeBuilder>,
    step_id: RecipeStepDraftId,
    ing_id: RecipeStepIngredientDraftId,
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
                class: "icon-button trash",
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
    // True once the user has explicitly arrow-navigated the popup. Until then
    // a highlight is only "real" if it's a prefix-extension of the typed text,
    // so TAB on `oil` doesn't silently replace it with `olive oil`.
    let mut navigated = use_signal(|| false);

    let filtered = filter_options(&options, &value);
    let popup_visible = open() && !filtered.is_empty();
    let highlight_idx = if popup_visible {
        highlight_offset().min(filtered.len() - 1)
    } else {
        0
    };

    let typed_lower = value.trim().to_lowercase();
    let highlighted_is_prefix = !typed_lower.is_empty()
        && filtered
            .get(highlight_idx)
            .map(|s| s.to_lowercase().starts_with(&typed_lower))
            .unwrap_or(false);
    let should_accept = popup_visible && (navigated() || highlighted_is_prefix);

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
                    navigated.set(false);
                    oninput.call(v);
                },
                onblur: move |_| {
                    open.set(false);
                    highlight_offset.set(0);
                    navigated.set(false);
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
                                    navigated.set(true);
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
                                    navigated.set(false);
                                } else {
                                    highlight_offset.set(cur - 1);
                                    navigated.set(true);
                                }
                            }
                            Key::Escape
                                if open() => {
                                e.prevent_default();
                                open.set(false);
                                highlight_offset.set(0);
                                navigated.set(false);
                            }
                            Key::Tab
                                // Accept the highlighted suggestion only if
                                // it's a prefix-extension of what the user
                                // typed, or they explicitly arrow-navigated.
                                // Otherwise let TAB advance focus without
                                // clobbering the typed text. Either way TAB
                                // keeps its default focus-advance behavior.
                                if should_accept => {
                                let i = highlight_offset().min(n - 1);
                                if let Some(s) = filtered.get(i) {
                                    oninput.call(s.clone());
                                }
                                open.set(false);
                                highlight_offset.set(0);
                                navigated.set(false);
                            }
                            Key::Enter => {
                                if has_command_modifier(&e.modifiers()) {
                                    return;
                                }
                                e.prevent_default();
                                if should_accept {
                                    let i = highlight_offset().min(n - 1);
                                    if let Some(s) = filtered.get(i) {
                                        oninput.call(s.clone());
                                    }
                                    open.set(false);
                                    highlight_offset.set(0);
                                    navigated.set(false);
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
                            active: i == highlight_idx && should_accept,
                            oninput,
                            open,
                            highlight_offset,
                            navigated,
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
    let mut scored: Vec<(bool, bool, usize, &String)> = options
        .iter()
        .filter_map(|o| {
            let lo = o.to_lowercase();
            lo.find(&needle).map(|p| {
                let not_exact = lo != needle;
                let not_prefix = !lo.starts_with(&needle);
                (not_exact, not_prefix, p, o)
            })
        })
        .collect();
    scored.sort_by_key(|(e, pr, p, _)| (*e, *pr, *p));
    scored
        .into_iter()
        .map(|(_, _, _, o)| o.clone())
        .take(8)
        .collect()
}

#[component]
fn AutocompleteItem(
    index: usize,
    text: String,
    active: bool,
    oninput: EventHandler<String>,
    open: Signal<bool>,
    highlight_offset: Signal<usize>,
    navigated: Signal<bool>,
) -> Element {
    let class = if active { "active" } else { "" };
    let text_for_click = text.clone();
    let mut open = open;
    let mut highlight_offset = highlight_offset;
    let mut navigated = navigated;
    rsx! {
        li {
            class: "{class}",
            onmousedown: move |e| {
                e.prevent_default();
                oninput.call(text_for_click.clone());
                open.set(false);
                highlight_offset.set(0);
                navigated.set(false);
            },
            onmousemove: move |_| {
                if highlight_offset() != index || !navigated() {
                    highlight_offset.set(index);
                    navigated.set(true);
                }
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
