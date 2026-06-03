use {
    api::{id::ShoppingListId, login_as_first, logout, routes::me, session::CurrentUser},
    dioxus::prelude::*,
    ui::{ThemeToggle, navbar::Navbar},
    views::{
        IngredientList, MealDetail, MealEdit, MealList, MealNew, RecipeDetail, RecipeEdit,
        RecipeList, RecipeNew, ShoppingListDetail, ShoppingListList, ShoppingListNew, TimerBar,
    },
};

mod client;
// FIXME
// pub mod local_storage;
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

const COLOR_CSS: Asset = asset!("/assets/color.css");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const ERROR_BANNER_JS: Asset = asset!("/assets/error-banner.js");
const FAVICON: Asset = asset!("/assets/favicon.svg");

/// Sets `<html data-theme>` before first paint so the palette in `color.css`
/// (`:root[data-theme="…"]`) is defined immediately and there's no flash of
/// unstyled colors. Uses the saved choice if present, otherwise the OS
/// preference. Runs inline in `<head>` so it executes before the body renders.
const THEME_SEED_JS: &str = r#"
(function () {
    try {
        var t = localStorage.getItem('theme');
        if (t !== 'light' && t !== 'dark') {
            t = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        document.documentElement.dataset.theme = t;
    } catch (e) {
        document.documentElement.dataset.theme = 'light';
    }
})();
"#;

fn main() {
    // Register the platform client.
    ui::initialize_client(Box::new(client::WebClient));

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
            api::db::migrate::run_migrations().await;

            let app_router = dioxus::server::router(App);
            Ok(app_router.layer(middleware::from_fn(api::log_server_errors)))
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
        // Runs before paint to set the theme attribute the palette keys off of.
        document::Script { {THEME_SEED_JS} }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Link { rel: "stylesheet", href: COLOR_CSS }
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
            ThemeToggle {}
        }
        main { id: "content", Outlet::<Route> {} }
        TimerBar {}
    }
}

#[component]
fn AuthControls() -> Element {
    let mut user = use_context::<CurrentUserCtx>();

    let inner = match user.read().as_ref() {
        Some(u) => {
            let name = u.name.clone();
            rsx! {
                span { class: "auth-user", "{name}" }
                button {
                    r#type: "button",
                    class: "linkish auth-logout",
                    onclick: move |_| {
                        spawn(async move {
                            if logout().await.is_ok() {
                                user.set(None);
                            }
                        });
                    },
                    "Log out"
                }
            }
        }
        // No real login UI yet: log in as the first user (see `login_as_first`).
        None => rsx! {
            button {
                r#type: "button",
                class: "linkish auth-login",
                onclick: move |_| {
                    spawn(async move {
                        if let Ok(current) = login_as_first().await {
                            user.set(Some(current));
                        }
                    });
                },
                "Log in"
            }
        },
    };

    rsx! {
        div { class: "auth-controls", {inner} }
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
