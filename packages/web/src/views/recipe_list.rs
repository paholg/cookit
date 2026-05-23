use crate::Route;
use api::list_recipes;
use dioxus::prelude::*;

#[component]
pub fn RecipeList() -> Element {
    let recipes = use_server_future(list_recipes)?;

    rsx! {
        header {
            class: "page-header",
            h1 { "Recipes" }
            Link {
                to: Route::RecipeNew {},
                class: "button",
                "+ New recipe"
            }
        }

        match recipes.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No recipes yet." }
            },
            Some(Ok(list)) => rsx! {
                ul {
                    class: "recipe-list",
                    for recipe in list {
                        li {
                            key: "{recipe.id}",
                            Link {
                                to: Route::RecipeDetail { id: recipe.id },
                                "{recipe.name}"
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading recipes: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
        }
    }
}
