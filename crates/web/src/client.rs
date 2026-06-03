use dioxus::prelude::*;

/// Flips `<html data-theme>` between light and dark and remembers the choice in
/// `localStorage` (read back on the next load by the seed script in `main`).
const TOGGLE_JS: &str = r#"
const root = document.documentElement;
const next = root.dataset.theme === 'dark' ? 'light' : 'dark';
root.dataset.theme = next;
try { localStorage.setItem('theme', next); } catch (e) {}
"#;

#[derive(Debug)]
pub struct WebClient;

impl ui::Client for WebClient {
    fn toggle_theme(&self) {
        document::eval(TOGGLE_JS);
    }
}
