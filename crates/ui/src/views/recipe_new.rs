use {
    crate::views::recipe_form::{RecipeForm, RecipeFormMode},
    api::{APP_NAME, RecipeBuilder, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        RecipeForm { initial: RecipeBuilder::new(), mode: RecipeFormMode::Create }
    }
}
