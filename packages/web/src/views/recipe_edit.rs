use crate::views::recipe_form::{RecipeDraft, RecipeForm, RecipeFormMode};
use api::get_recipe;
use dioxus::prelude::*;
#[component]
pub fn RecipeEdit(id: i64) -> Element {
    let recipe = use_server_future(move || get_recipe(id))?;
    match recipe.cloned() {
        Some(Ok(detail)) => {
            rsx! {
                RecipeForm {
                    initial: RecipeDraft::from_detail(detail),
                    mode: RecipeFormMode::Edit { id },
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                p { class: "error", "Error loading recipe: {e}" }
            }
        }
        None => {
            rsx! {
                p { "Loading..." }
            }
        }
    }
}
