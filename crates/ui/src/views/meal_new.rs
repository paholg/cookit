use {
    crate::views::meal_form::{MealForm, MealFormMode},
    api::{APP_NAME, MealBuilder, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn MealNew() -> Element {
    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        MealForm { initial: MealBuilder::new(), mode: MealFormMode::Create }
    }
}
