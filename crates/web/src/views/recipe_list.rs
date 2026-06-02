use {
    crate::{CurrentUserCtx, Route},
    api::list_recipes,
    dioxus::prelude::*,
};

#[component]
pub fn RecipeList() -> Element {
    let recipes = use_server_future(list_recipes)?;
    let user = use_context::<CurrentUserCtx>();
    let is_admin = user.read().as_ref().is_some_and(|u| u.is_admin());
    rsx! {
        document::Title { "CookIt!" }
        header { class: "page-header",
            h1 { "Recipes" }
            if is_admin {
                Link { to: Route::RecipeNew {}, class: "button primary", "+ New recipe" }
            }
        }
        match recipes.cloned() {
            Some(Ok(list)) if list.is_empty() => rsx! {
                p { class: "empty", "No recipes yet." }
            },
            Some(Ok(list)) => rsx! {
                ul { class: "recipe-list",
                    for recipe in list {
                        li { key: "{recipe.slug}",
                            Link {
                                to: Route::RecipeDetail {
                                    recipe_key: recipe.slug.clone(),
                                },
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
