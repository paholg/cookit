use {
    crate::{Field, Validated, client::client, use_field, use_form_validity},
    api::{
        auth::{authenticate_finish, authenticate_start},
        page_title,
    },
    db::Email,
    dioxus::prelude::*,
};

#[component]
pub fn Login() -> Element {
    let validity = use_form_validity();

    let email = use_field::<Email>();
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        error.set(None);

        // Guard: the button is disabled unless every field parses.
        let Ok(email) = email.value() else {
            return;
        };

        submitting.set(true);

        let result = async {
            let (user_id, rcr) = authenticate_start(email).await.map_err(|e| e.to_string())?;
            let pkc = client()
                .passkey_authenticate(rcr)
                .await
                .map_err(|e| e.to_string())?;
            authenticate_finish(user_id, pkc)
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        match result {
            // Switching to the user's book changes the host.
            Ok(current) => client().set_current_book(current.book.as_ref()),
            Err(e) => {
                error.set(Some(e));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "{page_title(\"Log in\")}" }
        header { class: "page-header",
            h1 { "Log in" }
        }

        form { class: "app-form", onsubmit: submit,
            label {
                "Email"
                Validated {
                    field: email,
                    render: move |mut f: Field<Email>| rsx! {
                        input {
                            r#type: "email",
                            value: f.text(),
                            oninput: move |e| f.set(e.value()),
                        }
                    },
                }
            }

            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }

            div { class: "form-actions",
                button {
                    r#type: "submit",
                    class: "primary",
                    disabled: submitting() || !validity.all_valid(),
                    if submitting() {
                        "Logging in..."
                    } else {
                        "Log in"
                    }
                }
            }
        }
    }
}
