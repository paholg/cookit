use {
    crate::ClientOnly,
    api::{
        APP_NAME, Ingredient, IngredientUpdate, Name, PositiveFloat,
        grocery_section::GrocerySection, id::IngredientId, list_ingredients, page_title,
        update_ingredient,
    },
    dioxus::prelude::*,
    dioxus_sdk::time::use_debounce,
    std::{str::FromStr, time::Duration},
};

#[derive(Clone, PartialEq)]
struct RowDraft {
    id: IngredientId,
    name: String,
    density: String,
    section: Option<GrocerySection>,
    saving: bool,
    error: Option<String>,
    /// Set on each edit, cleared when the in-flight save snapshots the row.
    /// Distinguishes "edited, save pending" from "saved and clean".
    dirty: bool,
    /// Whether at least one successful save has landed — so a never-touched row
    /// doesn't claim "Saved ✓".
    saved: bool,
}

impl RowDraft {
    fn from(i: &Ingredient) -> Self {
        Self {
            id: i.id,
            name: i.name.to_string(),
            density: i
                .density_g_per_ml
                .map(|d| format!("{d}"))
                .unwrap_or_default(),
            section: i.grocery_section,
            saving: false,
            error: None,
            dirty: false,
            saved: false,
        }
    }

    /// An ingredient still missing density or grocery section needs attention.
    fn is_incomplete(&self) -> bool {
        self.section.is_none()
    }

    fn to_payload(&self) -> Result<IngredientUpdate, String> {
        let name = Name::try_new(&self.name).map_err(|e| e.to_string())?;

        let density = match self.density.trim() {
            "" => None,
            t => {
                let v: f64 = t
                    .parse()
                    .map_err(|_| format!("`{t}` is not a valid density"))?;
                Some(PositiveFloat::try_new(v).map_err(|e| e.to_string())?)
            }
        };

        // The edit form always submits the full current state of the row, so
        // every field is `Some(...)` ("set to this value"); the nullable fields
        // wrap an inner `Option` (`Some(None)` clears the column).
        Ok(IngredientUpdate {
            id: self.id,
            name: Some(name),
            density_g_per_ml: Some(density),
            grocery_section: Some(self.section),
        })
    }
}

#[component]
pub fn IngredientList() -> Element {
    let server = use_server_future(list_ingredients)?;
    let mut rows = use_signal(Vec::<RowDraft>::new);
    use_effect(move || {
        if let Some(Ok(list)) = server.cloned() {
            let drafts: Vec<RowDraft> = list.iter().map(RowDraft::from).collect();
            if rows.read().is_empty() && !drafts.is_empty() {
                rows.set(drafts);
            }
        }
    });
    let incomplete_count = rows.read().iter().filter(|r| r.is_incomplete()).count();
    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        header { class: "page-header",
            h1 { "Ingredients" }
            if incomplete_count > 0 {
                span { class: "incomplete-summary", "⚠ {incomplete_count} need attention" }
            }
        }
        match server.cloned() {
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading ingredients: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No ingredients yet." }
            },
            Some(Ok(_)) => rsx! {
                ul { class: "ingredient-rows",
                    {
                        let row_count = rows.read().len();
                        rsx! {
                            for idx in 0..row_count {
                                IngredientRow { key: "{rows.read()[idx].id}", idx, rows }
                            }
                        }
                    }
                }
            },
        }
    }
}

#[component]
fn IngredientRow(idx: usize, rows: Signal<Vec<RowDraft>>) -> Element {
    // One debounce per row: each keystroke restarts the 500 ms countdown and the
    // row saves once editing pauses. Keeping it per-row means editing a second
    // row never cancels the first row's pending save.
    let mut autosave = use_debounce(Duration::from_millis(500), move |()| async move {
        let payload = {
            let r = rows.read();
            let Some(row) = r.get(idx) else { return };
            row.to_payload()
        };

        let payload = match payload {
            Ok(p) => p,
            Err(msg) => {
                if let Some(row) = rows.write().get_mut(idx) {
                    row.error = Some(msg);
                }
                return;
            }
        };

        // Snapshot taken — mark clean. A keystroke during the await sets `dirty`
        // again, so the row won't flash "Saved ✓" before the follow-up save.
        {
            let mut w = rows.write();
            let Some(row) = w.get_mut(idx) else { return };
            row.saving = true;
            row.error = None;
            row.dirty = false;
        }

        let result = update_ingredient(payload).await;

        let mut w = rows.write();
        let Some(row) = w.get_mut(idx) else { return };
        row.saving = false;
        match result {
            Ok(_updated) => row.saved = true,
            Err(e) => row.error = Some(e.to_string()),
        }
    });

    let row = rows.read().get(idx).cloned();
    let Some(row) = row else {
        return rsx! {};
    };
    let incomplete = row.is_incomplete();
    let settled = row.saved && !row.dirty && !row.saving && row.error.is_none();

    rsx! {
        li { class: if incomplete { "ingredient-row-card incomplete" } else { "ingredient-row-card" },
            div { class: "ingredient-row-grid",
                label {
                    span { class: "field-label", "Name" }
                    input {
                        r#type: "text",
                        value: "{row.name}",
                        oninput: move |e| {
                            let mut w = rows.write();
                            w[idx].name = e.value();
                            w[idx].dirty = true;
                            drop(w);
                            autosave.action(());
                        },
                    }
                }
                label {
                    span { class: "field-label",
                        "Density (g/ml)"
                    }
                    input {
                        r#type: "text",
                        inputmode: "decimal",
                        value: "{row.density}",
                        oninput: move |e| {
                            let mut w = rows.write();
                            w[idx].density = e.value();
                            w[idx].dirty = true;
                            drop(w);
                            autosave.action(());
                        },
                    }
                }
                label {
                    span { class: "field-label",
                        "Grocery section"
                        if row.section.is_none() {
                            span { class: "warn-tag", " ⚠" }
                        }
                    }
                    {
                        let current = row.section.map(|s| s.to_string()).unwrap_or_default();
                        rsx! {
                            ClientOnly {
                                select {
                                    value: "{current}",
                                    onchange: move |e| {
                                        let mut w = rows.write();
                                        w[idx].section = GrocerySection::from_str(&e.value()).ok();
                                        w[idx].dirty = true;
                                        drop(w);
                                        autosave.action(());
                                    },
                                    option { value: "", "—" }
                                    for section in GrocerySection::alphabetical_names() {
                                        option { value: "{section}", "{section}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "ingredient-row-actions",
                if let Some(err) = row.error.as_ref() {
                    span { class: "error inline", "{err}" }
                } else if row.saving {
                    span { class: "saved-tag", "Saving…" }
                } else if settled {
                    span { class: "saved-tag", "Saved ✓" }
                }
            }
        }
    }
}
