use dioxus::prelude::*;
use ui::Navbar;
use views::{
    Home, IngredientList, MealDetail, MealEdit, MealList, MealNew, RecipeDetail,
    RecipeEdit, RecipeList, RecipeNew,
};
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
    #[route("/meals")]
    MealList {},
    #[route("/meals/new")]
    MealNew {},
    #[route("/meals/:id")]
    MealDetail { id: i64 },
    #[route("/meals/:id/edit")]
    MealEdit { id: i64 },
    #[route("/ingredients")]
    IngredientList {},
}
const MAIN_CSS: Asset = asset!("/assets/main.css");
fn main() {
    dioxus::launch(App);
}
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        Router::<Route> {}
    }
}
#[component]
fn AppNavbar() -> Element {
    rsx! {
        Navbar {
            Link { to: Route::Home {}, "CookIt" }
            Link { to: Route::RecipeList {}, "Recipes" }
            Link { to: Route::MealList {}, "Meals" }
            Link { to: Route::IngredientList {}, "Ingredients" }
        }
        main { id: "content", Outlet::<Route> {} }
    }
}
