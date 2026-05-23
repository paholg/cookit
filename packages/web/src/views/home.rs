use crate::Route;
use dioxus::prelude::*;
#[component]
pub fn Home() -> Element {
    rsx! {
        section { class: "hero",
            h1 { "CookIt" }
            p { "Your self-hosted recipe collection." }
            Link { to: Route::RecipeList {}, class: "button", "Browse recipes" }
        }
    }
}
