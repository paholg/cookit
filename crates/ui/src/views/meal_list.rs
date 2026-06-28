use {
    crate::Route,
    api::{APP_NAME, Meal, list_meals, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn MealList() -> Element {
    let meals = use_server_future(list_meals)?;

    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        header { class: "page-header",
            h1 { "Meals" }
            Link { to: Route::MealNew {}, class: "button primary", "+ New meal" }
        }

        match meals.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No meals yet." }
            },
            Some(Ok(list)) => rsx! {
                ul { class: "card-list",
                    for meal in list {
                        MealRow { key: "{meal.slug}", meal }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading meals: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
        }
    }
}

#[component]
fn MealRow(meal: Meal) -> Element {
    let meal_key = meal.slug.to_string();

    rsx! {
        li {
            div { class: "card-row",
                Link {
                    to: Route::MealDetail { meal_key, tab: None },
                    "{meal.name}"
                }
            }
        }
    }
}
