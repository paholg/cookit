use crate::views::RecipeView;
use crate::{CurrentUserCtx, Route};
use api::get_recipe;
use dioxus::prelude::*;
use ui::icons::EditIcon;

#[component]
pub fn RecipeDetail(id: i64) -> Element {
    let recipe = use_server_future(move || get_recipe(id))?;

    let title = recipe
        .cloned()
        .and_then(|r| r.ok())
        .map(|d| d.recipe.name)
        .unwrap_or_else(|| "CookIt!".to_string());

    let user = use_context::<CurrentUserCtx>();
    let is_admin = user.read().clone().is_some_and(|u| u.is_admin);

    rsx! {
        document::Title { "{title}" }
        match recipe.cloned() {
            Some(Ok(detail)) => rsx! {
                header { class: "page-header",
                    h1 { "{detail.recipe.name}" }
                    if is_admin {
                        Link {
                            to: Route::RecipeEdit { id },
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
