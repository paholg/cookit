use {
    crate::Route,
    api::{
        MealBuilder, MealRecipeBuilder, Recipe, delete_meal,
        id::{DraftId, MealRecipeDraftId},
        list_recipes, upsert_meal,
    },
    dioxus::prelude::*,
    std::collections::HashSet,
    ui::{ClientOnly, icons::TrashIcon},
};

fn row_key(id: MealRecipeDraftId) -> String {
    format!("meal-recipe-{id}")
}

/// Append an empty recipe row (defaulting to ×1), returning its id.
fn push_new_row(draft: &mut Signal<MealBuilder>) -> MealRecipeDraftId {
    let mut d = draft.write();
    let id = DraftId::next(d.recipes.iter().map(|r| r.id));
    d.recipes.push(MealRecipeBuilder {
        id,
        recipe_slug: String::new(),
        multiplier: "1".to_string(),
    });
    id
}

/// True when every available recipe is already picked, so there's nothing left
/// to add.
fn all_recipes_used(draft: &MealBuilder, available: &[Recipe]) -> bool {
    let picked: HashSet<&str> = draft
        .recipes
        .iter()
        .map(|r| r.recipe_slug.as_str())
        .filter(|s| !s.is_empty())
        .collect();
    !available.is_empty() && available.iter().all(|r| picked.contains(r.slug.as_str()))
}

#[derive(Clone, PartialEq)]
pub enum MealFormMode {
    Create,
    Edit { meal_key: String },
}

#[component]
pub fn MealForm(initial: MealBuilder, mode: MealFormMode) -> Element {
    let mut draft = use_signal(|| initial.clone());
    let mut error = use_signal(|| None::<String>);
    let mut submitting = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let nav = use_navigator();

    let recipes = use_server_future(list_recipes)?;
    let available: Vec<Recipe> = match recipes.cloned() {
        Some(Ok(list)) => list,
        Some(Err(e)) => {
            return rsx! {
                p { class: "error", "Failed to load recipes: {e}" }
            };
        }
        None => Vec::new(),
    };

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
            match upsert_meal(payload).await {
                Ok(detail) => {
                    nav.push(Route::MealDetail {
                        meal_key: detail.meal.slug,
                        tab: None,
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
        MealFormMode::Create => "New meal",
        MealFormMode::Edit { .. } => "Edit meal",
    };
    let submit_label_idle = match mode {
        MealFormMode::Create => "Save meal",
        MealFormMode::Edit { .. } => "Save changes",
    };

    let rows: Vec<(MealRecipeDraftId, String)> = draft
        .read()
        .recipes
        .iter()
        .map(|r| (r.id, row_key(r.id)))
        .collect();
    let add_disabled = available.is_empty() || all_recipes_used(&draft.read(), &available);

    rsx! {
        header { class: "page-header",
            h1 { "{title}" }
            if let MealFormMode::Edit { meal_key } = mode.clone() {
                button {
                    r#type: "button",
                    class: "icon-button trash",
                    "aria-label": "Delete meal",
                    title: "Delete meal",
                    disabled: deleting() || submitting(),
                    onclick: move |_| {
                        if deleting() { return; }
                        let meal_key = meal_key.clone();
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
                            match delete_meal(meal_key).await {
                                Ok(()) => {
                                    nav.push(Route::MealList {});
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

        form { class: "app-form", onsubmit: submit,
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
            for (row_id, key) in rows.into_iter() {
                MealRecipeRow {
                    key: "{key}",
                    row_id,
                    draft,
                    recipes: available.clone(),
                }
            }
            button {
                r#type: "button",
                class: "secondary",
                disabled: add_disabled,
                onclick: move |_| {
                    push_new_row(&mut draft);
                },
                "+ Add recipe"
            }

            if let Some(err) = error.read().clone() {
                p { class: "error", "{err}" }
            }

            div { class: "form-actions",
                if let MealFormMode::Edit { meal_key } = mode.clone() {
                    Link {
                        to: Route::MealDetail { meal_key, tab: None },
                        class: "button secondary",
                        "Cancel"
                    }
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
    row_id: MealRecipeDraftId,
    draft: Signal<MealBuilder>,
    recipes: Vec<Recipe>,
) -> Element {
    let mut draft = draft;
    let Some(row) = draft
        .read()
        .recipes
        .iter()
        .find(|r| r.id == row_id)
        .cloned()
    else {
        return rsx! {};
    };

    let selected = row.recipe_slug.clone();
    let used_by_others: HashSet<String> = draft
        .read()
        .recipes
        .iter()
        .filter(|r| r.id != row_id)
        .map(|r| r.recipe_slug.clone())
        .filter(|s| !s.is_empty())
        .collect();

    rsx! {
        div { class: "meal-row",
            ClientOnly {
                select {
                    value: "{selected}",
                    oninput: move |e| {
                        let v = e.value();
                        let mut d = draft.write();
                        if let Some(r) = d.recipes.iter_mut().find(|r| r.id == row_id) {
                            r.recipe_slug = v;
                        }
                    },
                    option { value: "", "— pick a recipe —" }
                    for r in recipes.iter().filter(|r| !used_by_others.contains(&r.slug)) {
                        option {
                            value: "{r.slug}",
                            selected: row.recipe_slug == r.slug,
                            "{r.name}"
                        }
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
                class: "icon-button trash",
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
