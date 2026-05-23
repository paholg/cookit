use crate::views::meal_form::{MealDraft, MealForm, MealFormMode};
use dioxus::prelude::*;

#[component]
pub fn MealNew() -> Element {
    rsx! {
        MealForm {
            initial: MealDraft::empty(),
            mode: MealFormMode::Create,
        }
    }
}
