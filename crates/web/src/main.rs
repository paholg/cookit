use {
    api::me,
    dioxus::prelude::*,
    types::{CurrentUser, id::ShoppingListId},
    ui::navbar::Navbar,
    views::{
        IngredientList, MealDetail, MealEdit, MealList, MealNew, RecipeDetail, RecipeEdit,
        RecipeList, RecipeNew, ShoppingListDetail, ShoppingListList, ShoppingListNew, TimerBar,
    },
};

mod draft_id;
pub mod local_storage;
pub mod timers;
mod views;

pub use timers::RunningTimersCtx;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppNavbar)]
    #[redirect("/", || Route::RecipeList {})]
    #[route("/recipes")]
    RecipeList {},
    #[route("/recipes/new")]
    RecipeNew {},
    #[route("/recipes/:recipe_key")]
    RecipeDetail { recipe_key: String },
    #[route("/recipes/:recipe_key/edit")]
    RecipeEdit { recipe_key: String },
    #[route("/meals")]
    MealList {},
    #[route("/meals/new")]
    MealNew {},
    #[route("/meals/:meal_key?:tab")]
    MealDetail { meal_key: String, tab: Option<String> },
    #[route("/meals/:meal_key/edit")]
    MealEdit { meal_key: String },
    #[route("/ingredients")]
    IngredientList {},
    #[route("/shopping-lists")]
    ShoppingListList {},
    #[route("/shopping-lists/new")]
    ShoppingListNew {},
    #[route("/shopping-lists/:id")]
    ShoppingListDetail { id: ShoppingListId },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");
const ERROR_BANNER_JS: Asset = asset!("/assets/error-banner.js");
const FAVICON: Asset = asset!("/assets/favicon.svg");

fn main() {
    #[cfg(not(feature = "server"))]
    {
        // Route wasm Rust panics to `console.error` with a real stack trace.
        // Without this the browser swallows them silently.
        console_error_panic_hook::set_once();
        dioxus::launch(App);
    }

    #[cfg(feature = "server")]
    {
        use dioxus::server::axum::middleware;
        dioxus::serve(|| async {
            let auth_router = api::auth_router().await;
            let app_router = dioxus::server::router(App);
            Ok(auth_router
                .merge(app_router)
                .layer(middleware::from_fn(api::log_server_errors)))
        })
    }
}

/// Convenience signal for components that need to know the logged-in user.
/// Provided at the root via [`use_context_provider`] in [`App`].
pub type CurrentUserCtx = Signal<Option<CurrentUser>>;

#[component]
fn App() -> Element {
    let me_future = use_server_future(me)?;
    let user: Signal<Option<CurrentUser>> = use_signal(|| match me_future.cloned() {
        Some(Ok(u)) => u,
        _ => None,
    });
    use_context_provider(|| user);

    let mut timers: RunningTimersCtx = use_signal(Vec::new);
    use_context_provider(|| timers);

    // Hydrate from localStorage once, after the first client render. SSR sees
    // an empty bar (no DOM diff trouble) and the real list paints right after.
    // Also attach the audio primer so the WebAudio context gets resumed inside
    // every user gesture — required for the expired-timer beep to be audible.
    use_effect(move || {
        document::eval(timers::ATTACH_AUDIO_PRIMER_JS);
        let loaded = timers::load_from_storage();
        if !loaded.is_empty() {
            timers.set(loaded);
        }
    });

    rsx! {
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        // Loaded first so the listener is installed before any other JS runs.
        document::Script { src: ERROR_BANNER_JS }
        Router::<Route> {}
    }
}

#[component]
fn AppNavbar() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let logged_in = user.read().is_some();

    rsx! {
        Navbar {
            Link { to: Route::RecipeList {}, "CookIt" }
            Link { to: Route::RecipeList {}, "Recipes" }
            // Meals are available to everyone; unauthenticated users get a
            // localStorage-backed view via `api::meals`.
            Link { to: Route::MealList {}, "Meals" }
            Link { to: Route::ShoppingListList {}, "Shopping" }
            if logged_in {
                Link { to: Route::IngredientList {}, "Ingredients" }
            }
            AuthControls {}
        }
        main { id: "content", Outlet::<Route> {} }
        TimerBar {}
    }
}

#[component]
fn AuthControls() -> Element {
    let user = use_context::<CurrentUserCtx>();

    let inner = match user.read().as_ref() {
        Some(u) => {
            let name = u.name.clone();
            rsx! {
                span { class: "auth-user", "{name}" }
                form {
                    method: "post",
                    action: "/auth/logout",
                    class: "auth-logout",
                    button { r#type: "submit", class: "linkish", "Log out" }
                }
            }
        }
        None => login_controls(),
    };

    rsx! {
        div { class: "auth-controls", {inner} }
    }
}

fn login_controls() -> Element {
    rsx! {
        a { href: "/auth/login", class: "auth-login", "Log in" }
    }
}

/// Helper used by gated views to render a "please log in" message instead of
/// crashing on a 403.
pub fn require_login_or_message() -> Option<Element> {
    let user = use_context::<CurrentUserCtx>();
    if user.read().is_none() {
        Some(rsx! {
            p { class: "empty", "Please log in to view this page." }
        })
    } else {
        None
    }
}
