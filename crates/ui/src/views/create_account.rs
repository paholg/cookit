use {
    crate::{CurrentUserCtx, Route},
    api::{auth::create_user, page_title},
    db::{Email, Name},
    dioxus::prelude::*,
};

#[component]
pub fn CreateAccount() -> Element {
    let mut user = use_context::<CurrentUserCtx>();
    let nav = navigator();

    let mut name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        error.set(None);

        let name = match Name::try_from(name.read().trim()) {
            Ok(name) => name,
            Err(e) => {
                error.set(Some(format!("Invalid name: {e}")));
                return;
            }
        };

        let email = match Email::try_from(email.read().trim().to_string()) {
            Ok(email) => email,
            Err(e) => {
                error.set(Some(format!("Invalid email: {e}")));
                return;
            }
        };

        submitting.set(true);

        match create_user(name, email).await {
            Ok(current) => {
                user.set(current);
                nav.push(Route::CreatePasskey {});
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "{page_title(\"Create account\")}" }
        header { class: "page-header",
            h1 { "Create account" }
        }

        form { class: "app-form", onsubmit: submit,
            label {
                "Name"
                input {
                    r#type: "text",
                    value: name(),
                    oninput: move |e| name.set(e.value()),
                }
            }

            label {
                "Email"
                input {
                    r#type: "email",
                    value: email(),
                    oninput: move |e| email.set(e.value()),
                }
            }

            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }

            div { class: "form-actions",
                button { r#type: "submit", class: "primary", disabled: submitting(),
                    if submitting() {
                        "Creating..."
                    } else {
                        "Create account"
                    }
                }
            }
        }
    }
}
