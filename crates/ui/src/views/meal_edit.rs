use {
    crate::views::meal_form::{MealForm, MealFormMode},
    api::{APP_NAME, MealBuilder, get_meal, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn MealEdit(meal_key: String) -> Element {
    let meal = {
        let meal_key = meal_key.clone();
        use_server_future(move || get_meal(meal_key.clone()))?
    };

    let body = match meal.cloned() {
        Some(Ok(detail)) => rsx! {
            MealForm {
                initial: MealBuilder::from(detail),
                mode: MealFormMode::Edit {
                    meal_key: meal_key.clone(),
                },
            }
        },
        Some(Err(e)) => rsx! {
            p { class: "error", "Error loading meal: {e}" }
        },
        None => rsx! {
            p { "Loading..." }
        },
    };

    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        {body}
    }
}
