use {
    crate::{Route, client::client},
    api::{
        auth::{register_finish, register_start},
        page_title,
    },
    dioxus::prelude::*,
};

#[component]
pub fn CreatePasskey() -> Element {
    let nav = navigator();

    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let create = move |_| async move {
        submitting.set(true);
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
            Ok(()) => {
                nav.replace(Route::Home {});
            }
            Err(e) => {
                error.set(Some(e));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "{page_title(\"Create passkey\")}" }
        header { class: "page-header",
            h1 { "Create a passkey" }
        }

        p { "Add a passkey so you can sign in securely from this device." }

        if let Some(e) = error() {
            p { class: "error", "{e}" }
        }

        div { class: "form-actions",
            button {
                r#type: "button",
                class: "primary",
                disabled: submitting(),
                onclick: create,
                if submitting() {
                    "Waiting for passkey..."
                } else {
                    "Create passkey"
                }
            }
        }
    }
}
