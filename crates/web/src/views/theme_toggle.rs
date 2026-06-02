use {
    dioxus::prelude::*,
    ui::icons::{MoonIcon, SunIcon},
};

/// Flips `<html data-theme>` between light and dark and remembers the choice in
/// `localStorage` (read back on the next load by the seed script in `App`).
const TOGGLE_JS: &str = r#"
const root = document.documentElement;
const next = root.dataset.theme === 'dark' ? 'light' : 'dark';
root.dataset.theme = next;
try { localStorage.setItem('theme', next); } catch (e) {}
"#;

/// A sun/moon button that toggles the color theme.
///
/// Both icons are always rendered; `main.css` shows exactly one based on the
/// current `data-theme`, so SSR and the hydrated client agree and there's no
/// icon flicker on load.
#[component]
pub fn ThemeToggle() -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "icon-button theme-toggle",
            "aria-label": "Toggle light/dark theme",
            title: "Toggle light/dark theme",
            onclick: move |_| {
                document::eval(TOGGLE_JS);
            },
            span { class: "theme-icon sun", SunIcon {} }
            span { class: "theme-icon moon", MoonIcon {} }
        }
    }
}
