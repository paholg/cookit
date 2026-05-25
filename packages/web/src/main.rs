use api::me;
use dioxus::prelude::*;
use types::CurrentUser;
use ui::Navbar;
use views::{
    IngredientList, MealDetail, MealEdit, MealList, MealNew, RecipeDetail, RecipeEdit, RecipeList,
    RecipeNew,
};

mod draft_id;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppNavbar)]
    #[redirect("/", || Route::RecipeList {})]
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
const ERROR_BANNER_JS: Asset = asset!("/assets/error-banner.js");
const FAVICON: Asset = asset!("/assets/favicon.svg");

#[cfg(not(feature = "server"))]
fn main() {
    // Route wasm Rust panics to `console.error` with a real stack trace.
    // Without this the browser swallows them silently.
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[cfg(feature = "server")]
fn main() {
    use dioxus::server::axum::middleware;
    dioxus::serve(|| async {
        let auth_router = api::auth_router().await;
        let app_router = dioxus::server::router(App);
        Ok(auth_router
            .merge(app_router)
            .layer(middleware::from_fn(api::log_server_errors)))
    })
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
            if logged_in {
                Link { to: Route::IngredientList {}, "Ingredients" }
            }
            AuthControls {}
        }
        main { id: "content", Outlet::<Route> {} }
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

#[cfg(not(feature = "dev-auth"))]
fn login_controls() -> Element {
    rsx! {
        a { href: "/auth/login", class: "auth-login", "Log in" }
    }
}

#[cfg(feature = "dev-auth")]
fn login_controls() -> Element {
    rsx! { DevLoginSelect {} }
}

#[cfg(feature = "dev-auth")]
#[component]
fn DevLoginSelect() -> Element {
    let users_future = use_server_future(api::list_dev_users)?;
    let users = match users_future.cloned() {
        Some(Ok(list)) => list,
        _ => return rsx! { span { class: "auth-login", "…" } },
    };

    rsx! {
        form {
            method: "post",
            action: "/auth/dev-login",
            class: "auth-login",
            id: "dev-login-form",
            select {
                name: "user_id",
                required: true,
                onchange: move |_| {
                    let _ = document::eval("document.getElementById('dev-login-form').submit()");
                },
                option { value: "", disabled: true, selected: true, "Log in" }
                for u in users {
                    option {
                        value: "{u.id}",
                        if u.is_admin { "{u.name} (admin)" } else { "{u.name}" }
                    }
                }
            }
        }
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
