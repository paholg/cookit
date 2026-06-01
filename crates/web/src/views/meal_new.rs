use {
    crate::views::meal_form::{MealDraft, MealForm, MealFormMode},
    dioxus::prelude::*,
};
#[component]
pub fn MealNew() -> Element {
    rsx! {
        document::Title { "CookIt!" }
        MealForm { initial: MealDraft::empty(), mode: MealFormMode::Create }
    }
}
