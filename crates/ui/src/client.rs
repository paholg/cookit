use {async_trait::async_trait, dioxus::prelude::*, std::sync::OnceLock};

pub const BELL: Asset = asset!("/assets/bell.mp3");

#[async_trait(?Send)]
pub trait Client: Send + Sync + std::fmt::Debug {
    fn toggle_theme(&self);

    /// Acquire a screen wake lock. Returns `None` if the platform can't grant
    /// one (unsupported, denied, …). Dropping the returned guard releases the
    /// lock.
    async fn acquire_wake_lock(&self) -> Option<Box<dyn WakeLock>>;

    /// Prime the audio path inside a user gesture so a later timer-expiry bell
    /// is actually audible (browsers suspend audio created outside a gesture).
    /// No-op on platforms without that restriction.
    fn prime_audio(&self);

    /// Play the timer-expiry bell once. The timer bar repeats this on its own
    /// cadence while a timer is ringing; the client just plays a single tone.
    fn play_bell(&self);

    /// Move keyboard focus to the element tagged with the given focus key.
    /// Deferred to the next frame so it works for elements added in the same
    /// tick. No-op on platforms without a focusable view.
    fn focus_field(&self, key: &str);

    /// Resize the autogrow textarea tagged with the given key to fit its
    /// content. A shim for browsers without CSS `field-sizing: content`; a
    /// no-op where the platform sizes inputs itself.
    fn autogrow_textarea(&self, key: &str);

    /// Scroll the element named by the current location hash into view, if any.
    /// Lets a `#step-N` deep link land on the right step. No-op where there's
    /// no URL hash.
    fn scroll_to_hash(&self);
}

/// A held screen wake lock. Dropping it releases the lock.
#[async_trait(?Send)]
pub trait WakeLock {
    /// Resolves if the lock is released out from under us — e.g. the browser
    /// drops it when the tab is hidden. On platforms that hold the lock until
    /// it's dropped, this never resolves.
    async fn lost(&self);
}

static CLIENT: OnceLock<Box<dyn Client>> = OnceLock::new();

pub fn initialize_client(client: Box<dyn Client>) {
    CLIENT.set(client).unwrap();
}

pub fn client() -> &'static dyn Client {
    CLIENT.get().unwrap().as_ref()
}
