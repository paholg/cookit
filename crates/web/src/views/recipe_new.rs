use crate::views::recipe_form::{RecipeDraft, RecipeForm, RecipeFormMode};
use dioxus::prelude::*;
#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        document::Title { "CookIt!" }
        RecipeForm { initial: RecipeDraft::empty(), mode: RecipeFormMode::Create }
    }
}
