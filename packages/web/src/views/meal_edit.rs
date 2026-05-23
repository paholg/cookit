use crate::views::meal_form::{MealDraft, MealForm, MealFormMode};
use api::get_meal;
use dioxus::prelude::*;
#[component]
pub fn MealEdit(id: i64) -> Element {
    let meal = use_server_future(move || get_meal(id))?;
    match meal.cloned() {
        Some(Ok(detail)) => {
            rsx! {
                MealForm {
                    initial: MealDraft::from_detail(detail),
                    mode: MealFormMode::Edit { id },
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                p { class: "error", "Error loading meal: {e}" }
            }
        }
        None => {
            rsx! {
                p { "Loading..." }
            }
        }
    }
}
