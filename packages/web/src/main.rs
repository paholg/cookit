use dioxus::prelude::*;

use ui::Navbar;
use views::{Home, RecipeDetail, RecipeEdit, RecipeList, RecipeNew};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppNavbar)]
        #[route("/")]
        Home {},
        #[route("/recipes")]
        RecipeList {},
        #[route("/recipes/new")]
        RecipeNew {},
        #[route("/recipes/:id")]
        RecipeDetail { id: i64 },
        #[route("/recipes/:id/edit")]
        RecipeEdit { id: i64 },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1",
        }
        Router::<Route> {}
    }
}

#[component]
fn AppNavbar() -> Element {
    rsx! {
        Navbar {
            Link { to: Route::Home {}, "CookIt" }
            Link { to: Route::RecipeList {}, "Recipes" }
        }

        main {
            id: "content",
            Outlet::<Route> {}
        }
    }
}
