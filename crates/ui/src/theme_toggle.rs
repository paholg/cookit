use {
    crate::icons::{MoonIcon, SunIcon},
    dioxus::prelude::*,
};

/// A button that toggles the color theme.
#[component]
pub fn ThemeToggle() -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "icon-button theme-toggle",
            "aria-label": "Toggle light/dark theme",
            title: "Toggle light/dark theme",
            onclick: move |_| crate::client::client().toggle_theme(),
            span { class: "theme-icon sun", SunIcon {} }
            span { class: "theme-icon moon", MoonIcon {} }
        }
    }
}
