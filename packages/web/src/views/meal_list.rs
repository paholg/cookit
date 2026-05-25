use crate::{CurrentUserCtx, Route};
use api::meals::list_meals;
use dioxus::prelude::*;
use types::{CurrentUser, Meal};

fn can_modify(user: &Option<CurrentUser>, owner_id: Option<i64>) -> bool {
    match owner_id {
        // Local-storage meals have no owner — anyone viewing them owns them.
        None => true,
        Some(owner) => match user {
            Some(u) => u.is_admin || u.id == owner,
            None => false,
        },
    }
}

#[component]
pub fn MealList() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();

    // `use_resource` (not `use_server_future`) because the local-storage branch
    // runs in the browser; SSR can't see localStorage.
    let mut meals = use_resource(move || list_meals(authenticated));

    rsx! {
        document::Title { "CookIt!" }
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
                        MealRow {
                            key: "{meal.id}",
                            modifiable: can_modify(&user.read(), meal.user_id),
                            meal,
                            on_deleted: move |_| meals.restart(),
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

#[component]
fn MealRow(meal: Meal, modifiable: bool, on_deleted: EventHandler<()>) -> Element {
    let id = meal.id;

    rsx! {
        li {
            div { class: "meal-row-main",
                Link { to: Route::MealDetail { id }, "{meal.name}" }
            }
        }
    }
}
