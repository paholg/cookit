use {
    api::{
        Ingredient, IngredientUpdate,
        grocery_section::GrocerySection,
        helpers::{Name, PositiveFloat},
        id::IngredientId,
        list_ingredients, update_ingredient,
    },
    dioxus::prelude::*,
    std::str::FromStr,
    ui::ClientOnly,
};

#[derive(Clone, PartialEq)]
struct RowDraft {
    id: IngredientId,
    name: String,
    density: String,
    section: Option<GrocerySection>,
    saving: bool,
    error: Option<String>,
    pending_gen: u64,
    last_saved_gen: u64,
}

impl RowDraft {
    fn from(i: &Ingredient) -> Self {
        Self {
            id: i.id,
            name: i.name.0.clone(),
            density: i
                .density_g_per_ml
                .map(|d| format!("{}", d.0))
                .unwrap_or_default(),
            section: i.grocery_section,
            saving: false,
            error: None,
            pending_gen: 0,
            last_saved_gen: 0,
        }
    }

    /// An ingredient still missing density or grocery section needs attention.
    fn is_incomplete(&self) -> bool {
        parse_optional_density(&self.density).is_none() || self.section.is_none()
    }

    fn to_payload(&self) -> Result<IngredientUpdate, String> {
        let name = Name::parse(&self.name).map_err(|e| e.to_string())?;

        let density = match self.density.trim() {
            "" => None,
            t => {
                let v: f64 = t
                    .parse()
                    .map_err(|_| format!("`{t}` is not a valid density"))?;
                Some(PositiveFloat::parse(v).map_err(|e| e.to_string())?)
            }
        };

        Ok(IngredientUpdate {
            name,
            density_g_per_ml: density,
            grocery_section: self.section,
        })
    }
}

fn parse_optional_density(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
}

async fn autosave_delay() {
    #[cfg(feature = "web")]
    gloo_timers::future::TimeoutFuture::new(500).await;
}

fn trigger_autosave(idx: usize, mut rows: Signal<Vec<RowDraft>>) {
    let (id, this_gen) = {
        let mut w = rows.write();
        let Some(row) = w.get_mut(idx) else { return };
        row.pending_gen = row.pending_gen.wrapping_add(1);
        (row.id, row.pending_gen)
    };

    spawn(async move {
        autosave_delay().await;

        let payload = {
            let r = rows.read();
            let Some(row) = r.get(idx) else { return };
            if row.pending_gen != this_gen {
                return;
            }
            row.to_payload()
        };

        let payload = match payload {
            Ok(p) => p,
            Err(msg) => {
                if let Some(row) = rows.write().get_mut(idx) {
                    row.error = Some(msg);
                    row.saving = false;
                }
                return;
            }
        };

        {
            let mut w = rows.write();
            let Some(row) = w.get_mut(idx) else { return };
            row.saving = true;
            row.error = None;
        }

        let result = update_ingredient(id, payload).await;

        let mut w = rows.write();
        let Some(row) = w.get_mut(idx) else { return };
        row.saving = false;
        match result {
            Ok(_updated) => {
                row.last_saved_gen = this_gen;
            }
            Err(e) => {
                row.error = Some(e.to_string());
            }
        }
    });
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
        document::Title { "CookIt!" }
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
    let row = rows.read().get(idx).cloned();
    let Some(row) = row else {
        return rsx! {};
    };
    let incomplete = row.is_incomplete();
    let settled = row.last_saved_gen > 0
        && row.last_saved_gen == row.pending_gen
        && !row.saving
        && row.error.is_none();

    rsx! {
        li { class: if incomplete { "ingredient-row-card incomplete" } else { "ingredient-row-card" },
            div { class: "ingredient-row-grid",
                label {
                    span { class: "field-label", "Name" }
                    input {
                        r#type: "text",
                        value: "{row.name}",
                        oninput: move |e| {
                            rows.write()[idx].name = e.value();
                            trigger_autosave(idx, rows);
                        },
                    }
                }
                label {
                    span { class: "field-label",
                        "Density (g/ml)"
                        if incomplete && parse_optional_density(&row.density).is_none() {
                            span { class: "warn-tag", " ⚠" }
                        }
                    }
                    input {
                        r#type: "text",
                        inputmode: "decimal",
                        value: "{row.density}",
                        oninput: move |e| {
                            rows.write()[idx].density = e.value();
                            trigger_autosave(idx, rows);
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
                                        rows.write()[idx].section = GrocerySection::from_str(&e.value()).ok();
                                        trigger_autosave(idx, rows);
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
