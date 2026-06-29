use {
    crate::{
        FormatTimestamp, client::client, icons::TrashIcon, require_login_or_message, use_confirm,
    },
    api::{
        PasskeyInfo,
        auth::{delete_passkey, list_passkeys, register_finish, register_start},
        id::UserPasskeyId,
        page_title,
    },
    dioxus::prelude::*,
};

#[component]
pub fn Account() -> Element {
    if let Some(message) = require_login_or_message() {
        return message;
    }

    rsx! {
        document::Title { "{page_title(\"Account\")}" }
        header { class: "page-header",
            h1 { "Account" }
        }

        Passkeys {}
    }
}

#[component]
fn Passkeys() -> Element {
    let mut passkeys = use_server_future(list_passkeys)?;

    let mut adding = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let add = move |_| async move {
        adding.set(true);
        error.set(None);

        let result = async {
            let ccr = register_start().await.map_err(|e| e.to_string())?;
            let reg = client()
                .passkey_register(ccr)
                .await
                .map_err(|e| e.to_string())?;
            register_finish(reg).await.map_err(|e| e.to_string())
        }
        .await;

        match result {
            Ok(()) => passkeys.restart(),
            Err(e) => error.set(Some(e)),
        }

        adding.set(false);
    };

    rsx! {
        section {
            h2 { "Passkeys" }

            match passkeys.cloned() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    p { class: "empty", "No passkeys. Please add one so you can login!" }
                },
                Some(Ok(list)) => rsx! {
                    ul { class: "card-list",
                        for passkey in list {
                            PasskeyRow {
                                key: "{passkey.id}",
                                passkey,
                                on_deleted: move |_| passkeys.restart(),
                            }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    p { class: "error", "Error loading passkeys: {e}" }
                },
                None => rsx! {
                    p { "Loading..." }
                },
            }

            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }

            div { class: "form-actions",
                button {
                    r#type: "button",
                    class: "primary",
                    disabled: adding(),
                    onclick: add,
                    if adding() {
                        "Adding passkey..."
                    } else {
                        "Add passkey"
                    }
                }
            }
        }
    }
}

#[component]
fn PasskeyRow(passkey: PasskeyInfo, on_deleted: EventHandler<()>) -> Element {
    let id: UserPasskeyId = passkey.id;
    let mut deleting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let confirm = use_confirm();

    rsx! {
        li {
            div { class: "card-row",
                span { class: "row-label",
                    "Added "
                    FormatTimestamp { timestamp: passkey.created_at }
                }
                button {
                    r#type: "button",
                    class: "icon-button trash",
                    "aria-label": "Delete passkey",
                    title: "Delete passkey",
                    disabled: deleting(),
                    onclick: move |_| async move {
                        let confirmed = confirm
                            .show("Delete this passkey? You won't be able to sign in with it again.")
                            .await;
                        if !confirmed {
                            return;
                        }

                        deleting.set(true);
                        match delete_passkey(id).await {
                            Ok(()) => on_deleted.call(()),
                            Err(e) => error.set(Some(e.to_string())),
                        }
                        deleting.set(false);
                    },
                    TrashIcon {}
                }
            }
            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }
        }
    }
}
