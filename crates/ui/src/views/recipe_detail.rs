use {
    crate::{CurrentUserCtx, RecipeView, Route, WakeLockToggle, icons::EditIcon},
    api::{APP_NAME, get_recipe, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn RecipeDetail(recipe_key: String) -> Element {
    let recipe = {
        let key = recipe_key.clone();
        use_server_future(move || get_recipe(key.clone()))?
    };

    let title = recipe
        .cloned()
        .and_then(|r| r.ok())
        .map(|d| page_title(&d.recipe.name))
        .unwrap_or_else(|| page_title(APP_NAME));

    let user = use_context::<CurrentUserCtx>();
    let is_admin = user.read().is_admin();

    rsx! {
        document::Title { "{title}" }
        match recipe.cloned() {
            Some(Ok(detail)) => rsx! {
                header { class: "page-header",
                    h1 { "{detail.recipe.name}" }
                    div { class: "page-header-actions",
                        WakeLockToggle {}
                        if is_admin {
                            Link {
                                to: Route::RecipeEdit {
                                    recipe_key: recipe_key.clone(),
                                },
                                button {
                                    r#type: "button",
                                    class: "icon-button",
                                    "aria-label": "Edit recipe",
                                    title: "Edit recipe",
                                    EditIcon {}
                                }
                            }
                        }
                    }
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
