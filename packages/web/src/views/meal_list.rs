use crate::Route;
use api::list_meals;
use dioxus::prelude::*;

#[component]
pub fn MealList() -> Element {
    let meals = use_server_future(list_meals)?;

    rsx! {
        header {
            class: "page-header",
            h1 { "Meals" }
            Link {
                to: Route::MealNew {},
                class: "button",
                "+ New meal"
            }
        }

        match meals.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No meals yet." }
            },
            Some(Ok(list)) => rsx! {
                ul {
                    class: "recipe-list",
                    for meal in list {
                        li {
                            key: "{meal.id}",
                            Link {
                                to: Route::MealDetail { id: meal.id },
                                "{meal.name}"
                            }
                        }
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
