use {
    crate::views::recipe_form::{RecipeForm, RecipeFormMode},
    api::RecipeBuilder,
    dioxus::prelude::*,
};

#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        document::Title { "CookIt!" }
        RecipeForm { initial: RecipeBuilder::new(), mode: RecipeFormMode::Create }
    }
}
