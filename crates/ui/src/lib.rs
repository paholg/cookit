pub mod app;
pub mod client;
pub mod client_only;
pub mod components;
pub mod confirm;
pub mod error;
pub mod format;
pub mod icons;
pub mod navbar;
pub mod theme_toggle;
pub mod timer_bar;
pub mod timers;
pub mod validated;
pub mod views;
pub mod wake_toggle;

pub use {
    app::{App, CurrentUserCtx, Route, require_login_or_message},
    client::{BELL, Client, WakeLock, client, initialize_client},
    client_only::ClientOnly,
    confirm::{Confirm, ConfirmProvider, use_confirm},
    error::{Error, Result},
    theme_toggle::ThemeToggle,
    timer_bar::TimerBar,
    timers::RunningTimersCtx,
    validated::{Field, FormValidity, Validated, use_field, use_form_validity},
    views::{IngredientList, RecipeView, ShoppingListDetail},
    wake_toggle::WakeLockToggle,
};

pub const BASE_DOMAIN: &str = env!("BASE_DOMAIN");
