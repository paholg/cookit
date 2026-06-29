use {
    crate::Result,
    async_trait::async_trait,
    db::models::book::Book,
    dioxus::prelude::*,
    std::sync::OnceLock,
    webauthn_rs_proto::{
        CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
        RequestChallengeResponse,
    },
};

pub const BELL: Asset = asset!("/assets/bell.mp3");

#[async_trait(?Send)]
pub trait Client: Send + Sync + std::fmt::Debug {
    fn toggle_theme(&self);

    /// Acquire a screen wake lock. Returns `None` if the platform can't grant
    /// one (unsupported, denied, …). Dropping the returned guard releases the
    /// lock.
    async fn acquire_wake_lock(&self) -> Option<Box<dyn WakeLock>>;

    /// Play the timer-expiry bell.
    fn play_bell(&self);

    /// The user's time zone, defaulting to UTC if one cannot be determined.
    fn timezone(&self) -> jiff::tz::TimeZone;

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

    /// Set the currently active book.
    ///
    /// On the web, this will update the URL. On other platforms, it will likely
    /// set a header to be used with requests.
    fn set_current_book(&self, book: Option<&Book>);

    async fn passkey_register(
        &self,
        ccr: CreationChallengeResponse,
    ) -> Result<RegisterPublicKeyCredential>;

    async fn passkey_authenticate(
        &self,
        rcr: RequestChallengeResponse,
    ) -> Result<PublicKeyCredential>;
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
