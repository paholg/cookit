use {async_trait::async_trait, dioxus::document::eval};

#[derive(Debug)]
pub struct WebClient;

#[async_trait(?Send)]
impl ui::Client for WebClient {
    fn toggle_theme(&self) {
        // Flips `<html data-theme>` between light and dark and remembers the choice in
        // `localStorage` (read back on the next load by the seed script in `main`).
        eval(include_str!("js/toggle-theme.js"));
    }

    async fn acquire_wake_lock(&self) -> Option<Box<dyn ui::WakeLock>> {
        // Requests a screen wake lock and stashes the sentinel on `window` so the
        // lost/release helpers can find it. Any existing lock is released first, which
        // also cleans up a sentinel orphaned by a request that resolved after its
        // task was cancelled. Resolves `true` on success, `false` if the platform
        // refused (unsupported, denied, …).
        match eval(include_str!("js/wake-lock-acquire.js"))
            .join::<bool>()
            .await
        {
            Ok(true) => Some(Box::new(WebWakeLock)),
            _ => None,
        }
    }
}

/// Guard for the browser wake-lock sentinel stashed on `window`. Dropping it
/// releases the lock.
struct WebWakeLock;

#[async_trait(?Send)]
impl ui::WakeLock for WebWakeLock {
    async fn lost(&self) {
        // Resolves once the current sentinel fires its `release` event — i.e. the
        // browser dropped the lock (tab hidden, navigation, OS power policy). Resolves
        // immediately if there's no lock to watch.
        let _ = eval(include_str!("js/wake-lock-lost.js"))
            .join::<bool>()
            .await;
    }
}

impl Drop for WebWakeLock {
    fn drop(&mut self) {
        eval(include_str!("js/wake-lock-release.js"));
    }
}
