use {
    crate::views::recipe_form::{RecipeDraft, RecipeForm, RecipeFormMode},
    api::get_recipe,
    dioxus::prelude::*,
};

#[component]
pub fn RecipeEdit(recipe_key: String) -> Element {
    let recipe = {
        let recipe_key = recipe_key.clone();
        use_server_future(move || get_recipe(recipe_key.clone()))?
    };
    let body = match recipe.cloned() {
        Some(Ok(detail)) => rsx! {
            RecipeForm {
                initial: RecipeDraft::from_detail(detail),
                mode: RecipeFormMode::Edit {
                    recipe_key: recipe_key.clone(),
                },
            }
        },
        Some(Err(e)) => rsx! {
            p { class: "error", "Error loading recipe: {e}" }
        },
        None => rsx! {
            p { "Loading..." }
        },
    };

    rsx! {
        document::Title { "CookIt!" }
        {body}
    }
}
