use crate::{CurrentUserCtx, Route};
use api::meals::list_meals;
use dioxus::prelude::*;

#[component]
pub fn MealList() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();

    // `use_resource` (not `use_server_future`) because the local-storage branch
    // runs in the browser; SSR can't see localStorage.
    let meals = use_resource(move || list_meals(authenticated));

    rsx! {
        header { class: "page-header",
            h1 { "Meals" }
            Link { to: Route::MealNew {}, class: "button", "+ New meal" }
        }
        match meals.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No meals yet." }
            },
            Some(Ok(list)) => rsx! {
                ul { class: "recipe-list",
                    for meal in list {
                        li { key: "{meal.id}",
                            Link { to: Route::MealDetail { id: meal.id }, "{meal.name}" }
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
