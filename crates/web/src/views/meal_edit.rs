use {
    crate::views::meal_form::{MealDraft, MealForm, MealFormMode},
    api::meals::get_meal,
    dioxus::prelude::*,
};

#[component]
pub fn MealEdit(meal_key: String) -> Element {
    let meal = {
        let meal_key = meal_key.clone();
        use_resource(move || get_meal(meal_key.clone()))
    };

    let body = match meal.cloned() {
        Some(Ok(detail)) => rsx! {
            MealForm {
                initial: MealDraft::from_detail(detail),
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
        document::Title { "CookIt!" }
        {body}
    }
}
