use api::{list_ingredients, update_ingredient};
use dioxus::prelude::*;
use std::str::FromStr;
use strum::IntoEnumIterator;
use types::{GrocerySection, Ingredient, IngredientUpdate};
#[derive(Clone, PartialEq)]
struct RowDraft {
    id: i64,
    name: String,
    density: String,
    section: Option<GrocerySection>,
    ignore_density: bool,
    saving: bool,
    error: Option<String>,
    saved_tick: u32,
}
impl RowDraft {
    fn from(i: &Ingredient) -> Self {
        Self {
            id: i.id,
            name: i.name.clone(),
            density: i
                .density_g_per_ml
                .map(|d| format!("{d}"))
                .unwrap_or_default(),
            section: i.grocery_section,
            ignore_density: i.ignore_density,
            saving: false,
            error: None,
            saved_tick: 0,
        }
    }
    fn snapshot(&self) -> Ingredient {
        Ingredient {
            id: self.id,
            name: self.name.clone(),
            density_g_per_ml: parse_optional_density(&self.density),
            grocery_section: self.section,
            ignore_density: self.ignore_density,
        }
    }
    fn to_payload(&self) -> Result<IngredientUpdate, String> {
        if self.name.trim().is_empty() {
            return Err("name is required".into());
        }
        let density = if self.density.trim().is_empty() {
            None
        } else {
            Some(
                self.density
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("`{}` is not a valid density", self.density.trim()))?,
            )
        };
        Ok(IngredientUpdate {
            name: self.name.clone(),
            density_g_per_ml: density,
            grocery_section: self.section,
            ignore_density: self.ignore_density,
        })
    }
}
fn parse_optional_density(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
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
    let incomplete_count = rows
        .read()
        .iter()
        .filter(|r| r.snapshot().is_incomplete())
        .count();
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
    let incomplete = row.snapshot().is_incomplete();
    let save = move |_| {
        let snapshot = rows.read()[idx].clone();
        let payload = match snapshot.to_payload() {
            Ok(p) => p,
            Err(msg) => {
                rows.write()[idx].error = Some(msg);
                return;
            }
        };
        let id = snapshot.id;
        {
            let mut w = rows.write();
            w[idx].saving = true;
            w[idx].error = None;
        }
        spawn(async move {
            match update_ingredient(id, payload).await {
                Ok(()) => {
                    let mut w = rows.write();
                    w[idx].saving = false;
                    w[idx].saved_tick = w[idx].saved_tick.wrapping_add(1);
                }
                Err(e) => {
                    let mut w = rows.write();
                    w[idx].saving = false;
                    w[idx].error = Some(e.to_string());
                }
            }
        });
    };
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
                        },
                    }
                }
                label {
                    span { class: "field-label",
                        "Density (g/ml)"
                        if incomplete && row.snapshot().density_g_per_ml.is_none() && !row.ignore_density {
                            span { class: "warn-tag", " ⚠" }
                        }
                    }
                    input {
                        r#type: "text",
                        inputmode: "decimal",
                        value: "{row.density}",
                        disabled: row.ignore_density,
                        oninput: move |e| {
                            rows.write()[idx].density = e.value();
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
                            select {
                                value: "{current}",
                                onchange: move |e| {
                                    rows.write()[idx].section = GrocerySection::from_str(&e.value()).ok();
                                },
                                option { value: "", "—" }
                                for section in GrocerySection::iter() {
                                    option { value: "{section}", "{section}" }
                                }
                            }
                        }
                    }
                }
                label { class: "checkbox-label",
                    input {
                        r#type: "checkbox",
                        checked: row
                                                                                                                                                                                .ignore_density,
                        oninput: move |e| {
                            rows.write()[idx].ignore_density = e.checked();
                        },
                    }
                    span { "Ignore density (e.g. eggs, lemons)" }
                }
            }
            div { class: "ingredient-row-actions",
                button {
                    r#type: "button",
                    class: "primary",
                    disabled: row.saving,
                    onclick: save,
                    if row.saving {
                        "Saving..."
                    } else {
                        "Save"
                    }
                }
                if let Some(err) = row.error.as_ref() {
                    span { class: "error inline", "{err}" }
                } else if row.saved_tick > 0 {
                    span { class: "saved-tag", "Saved ✓" }
                }
            }
        }
    }
}
