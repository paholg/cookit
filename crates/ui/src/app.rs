use {
    crate::{
        ConfirmProvider, RunningTimersCtx, ThemeToggle, TimerBar,
        client::{BELL, client},
        icons::MenuIcon,
        navbar::Navbar,
        timers::{self, RunningTimer},
        timezone::initialize_timezone,
        views::{
            Account, CreateAccount, CreateBook, Home, IngredientList, Login, MealDetail, MealEdit,
            MealList, MealNew, RecipeDetail, RecipeEdit, RecipeList, RecipeNew, ShoppingListDetail,
            ShoppingListList, ShoppingListNew,
        },
    },
    api::{APP_NAME, Current, id::ShoppingListId, logout, page_title, routes::me},
    dioxus::prelude::*,
    dioxus_primitives::dropdown_menu::{
        DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
    },
    dioxus_sdk::storage::{LocalStorage, use_synced_storage},
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppNavbar)]
    #[route("/")]
    Home {},
    #[route("/create-account")]
    CreateAccount {},
    #[route("/account")]
    Account {},
    #[route("/create-book")]
    CreateBook {},
    #[route("/login")]
    Login {},
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
pub type CurrentUserCtx = Signal<Current>;

#[component]
pub fn App() -> Element {
    let me_future = use_server_future(me)?;
    let user: Signal<Current> = use_signal(|| match me_future.cloned() {
        Some(Ok(u)) => u,
        _ => Current::none(),
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

    initialize_timezone();

    rsx! {
        // Runs before paint to set the theme attribute the palette keys off of.
        document::Script { {THEME_SEED_JS} }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Link { rel: "stylesheet", href: COLOR_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        // Loaded first so the listener is installed before any other JS runs.
        document::Script { src: ERROR_BANNER_JS }

        audio { id: "timer-bell", src: BELL, preload: "auto" }

        ConfirmProvider {
            Router::<Route> {}
        }
    }
}

#[component]
fn AppNavbar() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let logged_in = user.read().is_logged_in();
    let has_book = user.read().book.is_some();

    rsx! {
        Navbar {
            div { class: "nav-links",
                Link { to: Route::Home {}, "{page_title(APP_NAME)}" }
                if has_book {
                    Link { to: Route::RecipeList {}, "Recipes" }
                    Link { to: Route::MealList {}, "Meals" }

                    if logged_in {
                        Link { to: Route::ShoppingListList {}, "Shopping" }
                        Link { to: Route::IngredientList {}, "Ingredients" }
                    }
                }
            }
            div { class: "nav-actions",
                AuthControls {}
                ThemeToggle {}
            }
        }
        main { id: "content", Outlet::<Route> {} }
        TimerBar {}
    }
}

#[component]
fn AuthControls() -> Element {
    let user = use_context::<CurrentUserCtx>();

    let inner = if user.read().is_logged_in() {
        rsx! { MainMenu {} }
    } else {
        rsx! {
            Link { to: Route::Login {}, class: "linkish auth-login", "Log in" }
        }
    };

    rsx! {
        div { class: "auth-controls", {inner} }
    }
}

#[component]
fn MainMenu() -> Element {
    let nav = navigator();

    rsx! {
        DropdownMenu { class: "main-menu",
            DropdownMenuTrigger {
                class: "icon-button main-menu-trigger",
                aria_label: "Account menu",
                MenuIcon {}
            }
            DropdownMenuContent { class: "main-menu-content",
                DropdownMenuItem::<()> {
                    value: (),
                    index: 0usize,
                    class: "main-menu-item",
                    on_select: move |_| {
                        nav.push(Route::Account {});
                    },
                    "Account"
                }
                DropdownMenuItem::<()> {
                    value: (),
                    index: 1usize,
                    class: "main-menu-item",
                    on_select: |_| {
                        spawn(async move {
                            if logout().await.is_ok() {
                                client().set_current_book(None);
                            }
                        });
                    },
                    "Log out"
                }
            }
        }
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
