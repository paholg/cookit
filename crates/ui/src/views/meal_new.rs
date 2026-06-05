use {
    crate::views::meal_form::{MealForm, MealFormMode},
    api::{APP_NAME, MealBuilder},
    dioxus::prelude::*,
};

#[component]
pub fn MealNew() -> Element {
    rsx! {
        document::Title { "{APP_NAME}" }
        MealForm { initial: MealBuilder::new(), mode: MealFormMode::Create }
    }
}
