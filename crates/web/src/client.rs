use {
    async_trait::async_trait,
    db::models::book::Book,
    dioxus::document::eval,
    snafu::OptionExt,
    ui::{BASE_DOMAIN, Client, Error, error::OtherSnafu},
    web_sys::js_sys::futures::JsFuture,
    webauthn_rs_proto::{
        CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
        RequestChallengeResponse,
    },
};

#[derive(Debug)]
pub struct WebClient;

#[async_trait(?Send)]
impl Client for WebClient {
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

    fn play_bell(&self) {
        eval(include_str!("js/play-bell.js"));
    }

    fn timezone(&self) -> jiff::tz::TimeZone {
        iana_timezone()
            .and_then(|name| jiff::tz::TimeZone::get(&name).ok())
            .unwrap_or(jiff::tz::TimeZone::UTC)
    }

    fn focus_field(&self, key: &str) {
        let safe = key.replace('"', "");

        eval(&format!(
            "requestAnimationFrame(() => {{ const el = \
             document.querySelector('[data-focus-key=\"{safe}\"]'); if (el) el.focus(); }})"
        ));
    }

    fn autogrow_textarea(&self, key: &str) {
        // Firefox <152 doesn't support CSS `field-sizing: content`, so size the
        // textarea from JS. Once Firefox 152+ is widespread the CSS rule alone
        // suffices and this can be removed. Queries `data-autogrow` (separate
        // from `data-focus-key`) so the focus target and the autogrow target
        // can be different elements.
        let safe = key.replace('"', "");

        eval(&format!(
            "requestAnimationFrame(() => {{ const el = \
             document.querySelector('[data-autogrow=\"{safe}\"]'); if (el) {{ el.style.height = \
             'auto'; el.style.height = el.scrollHeight + 'px'; }} }})"
        ));
    }

    fn scroll_to_hash(&self) {
        eval(include_str!("js/scroll-to-hash.js"));
    }

    fn set_current_book(&self, book: Option<&Book>) {
        let hostname = match book {
            Some(book) => format!("{}.{}", book.slug, BASE_DOMAIN),
            None => BASE_DOMAIN.to_string(),
        };

        // Switch hosts and land on the new book's home.
        eval(&format!(
            "const u = new URL(window.location.href); u.hostname = \"{hostname}\"; u.pathname = \
             \"/\"; window.location.href = u.toString();"
        ));
    }

    async fn passkey_register(
        &self,
        ccr: CreationChallengeResponse,
    ) -> ui::Result<RegisterPublicKeyCredential> {
        let c_options: web_sys::CredentialCreationOptions = ccr.into();

        let promise = web_sys::window()
            .context(OtherSnafu {
                msg: "no browser window",
            })?
            .navigator()
            .credentials()
            .create_with_options(&c_options)
            .map_err(|e| Error::Other {
                msg: format!("failed to create passkey prompt: {e:?}"),
            })?;

        let credential = JsFuture::from(promise).await.map_err(|e| Error::Other {
            msg: format!("prompt cancelled: {e:?}"),
        })?;

        Ok(RegisterPublicKeyCredential::from(
            web_sys::PublicKeyCredential::from(credential),
        ))
    }

    async fn passkey_authenticate(
        &self,
        rcr: RequestChallengeResponse,
    ) -> ui::Result<PublicKeyCredential> {
        let c_options: web_sys::CredentialRequestOptions = rcr.into();

        let promise = web_sys::window()
            .context(OtherSnafu {
                msg: "no browser window",
            })?
            .navigator()
            .credentials()
            .get_with_options(&c_options)
            .map_err(|e| Error::Other {
                msg: format!("failed to create passkey prompt: {e:?}"),
            })?;

        let assertion = JsFuture::from(promise).await.map_err(|e| Error::Other {
            msg: format!("prompt cancelled: {e:?}"),
        })?;

        Ok(PublicKeyCredential::from(
            web_sys::PublicKeyCredential::from(assertion),
        ))
    }
}

fn iana_timezone() -> Option<String> {
    use web_sys::js_sys::{Array, Intl, JsString, Object, Reflect};

    let format = Intl::DateTimeFormat::new(&Array::new(), &Object::new());
    let resolved = format.resolved_options();

    Reflect::get(&resolved, &JsString::from("timeZone"))
        .ok()?
        .as_string()
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
