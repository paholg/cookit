use {
    crate::views::recipe_form::{RecipeForm, RecipeFormMode},
    api::{APP_NAME, RecipeBuilder},
    dioxus::prelude::*,
};

#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        document::Title { "{APP_NAME}" }
        RecipeForm { initial: RecipeBuilder::new(), mode: RecipeFormMode::Create }
    }
}
