use {
    crate::{
        ConfirmProvider, RunningTimersCtx, ThemeToggle, TimerBar,
        client::client,
        navbar::Navbar,
        timers::{self, RunningTimer},
        views::{
            IngredientList, MealDetail, MealEdit, MealList, MealNew, RecipeDetail, RecipeEdit,
            RecipeList, RecipeNew, ShoppingListDetail, ShoppingListList, ShoppingListNew,
        },
    },
    api::{APP_NAME, AuthUser, id::ShoppingListId, login_as_first, logout, page_title, routes::me},
    dioxus::prelude::*,
    dioxus_sdk::storage::{LocalStorage, use_synced_storage},
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
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

/// Convenience signal for components that need to know the logged-in user.
/// Provided at the root via [`use_context_provider`] in [`App`].
pub type CurrentUserCtx = Signal<AuthUser>;

#[component]
pub fn App() -> Element {
    let me_future = use_server_future(me)?;
    let user: Signal<AuthUser> = use_signal(|| match me_future.cloned() {
        Some(Ok(u)) => u,
        _ => AuthUser::none(),
    });
    use_context_provider(|| user);

    // `use_synced_storage` returns a `Signal` backed by `localStorage`: it
    // hydrates from storage on the client (SSR sees an empty bar, so there's no
    // DOM diff trouble), persists on every write, and syncs across tabs.
    let timers: RunningTimersCtx = use_synced_storage::<LocalStorage, Vec<RunningTimer>>(
        timers::STORAGE_KEY.to_string(),
        Vec::new,
    );
    use_context_provider(|| timers);

    // Prime the audio path so the WebAudio context gets resumed inside every
    // user gesture — required for the expired-timer bell to be audible.
    use_effect(move || {
        client().prime_audio();
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
        ConfirmProvider {
            Router::<Route> {}
        }
    }
}

#[component]
fn AppNavbar() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let logged_in = user.read().is_logged_in();

    rsx! {
        Navbar {
            Link { to: Route::RecipeList {}, "{page_title(APP_NAME)}" }
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

    let inner = match user.read().user.as_ref() {
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
                                user.set(AuthUser::none());
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
                            user.set(current);
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
    if !user.read().is_logged_in() {
        Some(rsx! {
            p { class: "empty", "Please log in to view this page." }
        })
    } else {
        None
    }
}
