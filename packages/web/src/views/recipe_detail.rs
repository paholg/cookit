use crate::Route;
use crate::views::RecipeView;
use api::get_recipe;
use dioxus::prelude::*;

#[component]
pub fn RecipeDetail(id: i64) -> Element {
    let recipe = use_server_future(move || get_recipe(id))?;

    let title = recipe
        .cloned()
        .and_then(|r| r.ok())
        .map(|d| d.recipe.name)
        .unwrap_or_else(|| "CookIt!".to_string());

    rsx! {
        document::Title { "{title}" }
        match recipe.cloned() {
            Some(Ok(detail)) => rsx! {
                header { class: "page-header",
                    h1 { "{detail.recipe.name}" }
                    Link { to: Route::RecipeEdit { id }, class: "button-link", "Edit" }
                }
                RecipeView { detail, multiplier: 1.0 }
            },
            Some(Err(e)) => rsx! {
                p { class: "error", "Error loading recipe: {e}" }
            },
            None => rsx! {
                p { "Loading..." }
            },
        }
    }
}
