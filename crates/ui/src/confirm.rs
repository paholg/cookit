//! App-wide confirmation dialog. Replaces the old `Client::confirm` (a JS
//! `window.confirm`) with the vendored Dioxus [`AlertDialog`].
//!
//! [`ConfirmProvider`] mounts the dialog once near the app root and provides a
//! [`Confirm`] handle. Any descendant calls [`use_confirm`] and awaits
//! `confirm.show(msg)`, which resolves `true` when the user confirms and
//! `false` on cancel / escape / backdrop — matching the ergonomics of the old
//! `client().confirm(msg).await`.

use {
    crate::components::alert_dialog::{
        AlertDialog, AlertDialogAction, AlertDialogActions, AlertDialogCancel,
        AlertDialogDescription, AlertDialogTitle,
    },
    dioxus::prelude::*,
    futures::channel::oneshot,
};

/// A pending confirmation: the prompt to show and the channel to answer it on.
struct ConfirmRequest {
    message: String,
    responder: oneshot::Sender<bool>,
}

/// Handle for requesting confirmation. Obtain it with [`use_confirm`], then
/// `confirm.show(message).await`.
#[derive(Clone, Copy)]
pub struct Confirm {
    request: Signal<Option<ConfirmRequest>>,
}

impl Confirm {
    /// Show the dialog with `message` and resolve to the user's choice
    /// (`true` = confirmed). Cancelling, Escape, or a backdrop click resolve
    /// `false`.
    pub async fn show(mut self, message: impl Into<String>) -> bool {
        let (responder, answer) = oneshot::channel();

        self.request.set(Some(ConfirmRequest {
            message: message.into(),
            responder,
        }));

        answer.await.unwrap_or(false)
    }
}

/// Read the [`Confirm`] handle provided by [`ConfirmProvider`].
pub fn use_confirm() -> Confirm {
    use_context()
}

/// Mounts the confirmation [`AlertDialog`] once and provides a [`Confirm`]
/// handle to descendants. Place near the app root, wrapping the routes.
#[component]
pub fn ConfirmProvider(children: Element) -> Element {
    let mut request = use_signal(|| None::<ConfirmRequest>);
    use_context_provider(|| Confirm { request });

    // The vendored `AlertDialogAction`/`Cancel` run their internal close
    // (`set_open(false)` → our `on_open_change`) *before* the button's
    // `on_click`. So the button records its answer here, and `on_open_change`
    // resolves on a deferred task that runs once those synchronous handlers
    // finish — at which point `answer` holds the button's choice (or stays
    // `None` for escape/backdrop, which we read as "cancelled").
    let mut answer = use_signal(|| None::<bool>);

    let message = request.read().as_ref().map(|r| r.message.clone());
    let open = message.is_some();
    let description = message.unwrap_or_default();

    rsx! {
        {children}

        AlertDialog {
            open: Some(open),
            on_open_change: move |opened: bool| {
                if !opened {
                    spawn(async move {
                        let confirmed = answer.write().take().unwrap_or(false);
                        if let Some(req) = request.write().take() {
                            let _ = req.responder.send(confirmed);
                        }
                    });
                }
            },
            AlertDialogTitle { "Confirm" }
            AlertDialogDescription { "{description}" }
            AlertDialogActions {
                AlertDialogCancel {
                    on_click: move |_| answer.set(Some(false)),
                    "Cancel"
                }
                AlertDialogAction {
                    on_click: move |_| answer.set(Some(true)),
                    "Delete"
                }
            }
        }
    }
}
