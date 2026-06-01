use {
    crate::{CurrentUserCtx, Route},
    api::meals::list_meals,
    dioxus::prelude::*,
    types::Meal,
};

#[component]
pub fn MealList() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let authenticated = user.read().is_some();

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
                            key: "{meal.slug}",
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
fn MealRow(meal: Meal, on_deleted: EventHandler<()>) -> Element {
    let meal_key = meal.slug.clone();

    rsx! {
        li {
            div { class: "meal-row-main",
                Link {
                    to: Route::MealDetail {
                        meal_key,
                        tab: None,
                    },
                    "{meal.name}"
                }
            }
        }
    }
}
