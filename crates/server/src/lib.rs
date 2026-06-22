pub mod config;
pub mod conn;
pub mod dev;
mod error;
mod middleware;
mod migrate;
mod models;
mod request_context;
mod session;
pub mod webauthn;

pub use {
    error::{Error, Result},
    middleware::log_server_errors,
    models::*,
    request_context::RequestContext,
    session::{AuthUser, CookitAuthSession, install},
};

pub fn serve(app: fn() -> dioxus::core::Element) -> ! {
    use dioxus::server::axum::middleware;

    dioxus::serve(move || async move {
        migrate::run_migrations().await;

        let app_router = dioxus::server::router(app);
        let app_router = session::install(app_router).await;
        Ok(app_router.layer(middleware::from_fn(log_server_errors)))
    })
}
