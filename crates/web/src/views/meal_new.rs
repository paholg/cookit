use {
    crate::views::meal_form::{MealForm, MealFormMode},
    api::MealBuilder,
    dioxus::prelude::*,
};

#[component]
pub fn MealNew() -> Element {
    rsx! {
        document::Title { "CookIt!" }
        MealForm { initial: MealBuilder::new(), mode: MealFormMode::Create }
    }
}
