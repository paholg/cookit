use {
    crate::{
        Field, FormatTimestamp, Validated,
        client::client,
        components::dialog::{Dialog, DialogTitle},
        icons::TrashIcon,
        require_login_or_message, use_confirm, use_field, use_form_validity,
    },
    api::{
        PasskeyInfo,
        auth::{delete_passkey, list_passkeys, register_finish, register_start},
        id::UserPasskeyId,
        page_title,
    },
    db::Name,
    dioxus::prelude::*,
    webauthn_rs_proto::RegisterPublicKeyCredential,
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

    let validity = use_form_validity();
    let mut name = use_field::<Name>();

    let mut pending_credential: Signal<Option<RegisterPublicKeyCredential>> = use_signal(|| None);

    let mut adding = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let register_passkey = move |_| async move {
        adding.set(true);
        error.set(None);

        let result = async {
            let ccr = register_start().await.map_err(|e| e.to_string())?;
            client()
                .passkey_register(ccr)
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        match result {
            Ok(reg) => pending_credential.set(Some(reg)),
            Err(e) => error.set(Some(e)),
        }

        adding.set(false);
    };

    let save_passkey = move |e: FormEvent| async move {
        e.prevent_default();

        let (Some(reg), Ok(name_value)) = (pending_credential(), name.value()) else {
            return;
        };

        saving.set(true);
        error.set(None);

        match register_finish(name_value, reg).await {
            Ok(()) => {
                name.set(String::new());
                pending_credential.set(None);
                passkeys.restart();
            }
            Err(e) => error.set(Some(e.to_string())),
        }

        saving.set(false);
    };

    let mut cancel = move || {
        pending_credential.set(None);
        name.set(String::new());
        error.set(None);
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

            if pending_credential().is_none() {
                if let Some(e) = error() {
                    p { class: "error", "{e}" }
                }
            }

            div { class: "form-actions",
                button {
                    r#type: "button",
                    class: "primary",
                    disabled: adding(),
                    onclick: register_passkey,
                    if adding() {
                        "Adding passkey..."
                    } else {
                        "Add passkey"
                    }
                }
            }

            // Modal to name the passkey.
            Dialog {
                open: Some(pending_credential().is_some()),
                on_open_change: move |opened: bool| {
                    if !opened && !saving() {
                        cancel();
                    }
                },
                DialogTitle { "Name your passkey" }
                form { class: "app-form", onsubmit: save_passkey,
                    Validated {
                        field: name,
                        render: move |mut f: Field<Name>| rsx! {
                            input {
                                r#type: "text",
                                placeholder: "Name",
                                value: f.text(),
                                oninput: move |e| f.set(e.value()),
                            }
                        },
                    }

                    if let Some(e) = error() {
                        p { class: "error", "{e}" }
                    }

                    div { class: "form-actions",
                        button {
                            r#type: "button",
                            class: "button",
                            disabled: saving(),
                            onclick: move |_| cancel(),
                            "Cancel"
                        }
                        button {
                            r#type: "submit",
                            class: "button primary",
                            disabled: saving() || !validity.all_valid(),
                            if saving() {
                                "Saving..."
                            } else {
                                "Save"
                            }
                        }
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
                    "{passkey.name}"
                    span { class: "kbd-hint",
                        "Added "
                        FormatTimestamp { timestamp: passkey.created_at }
                    }
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
