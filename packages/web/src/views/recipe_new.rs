use crate::views::recipe_form::{RecipeDraft, RecipeForm, RecipeFormMode};
use dioxus::prelude::*;

#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        RecipeForm {
            initial: RecipeDraft::empty(),
            mode: RecipeFormMode::Create,
        }
    }
}
