use {
    crate::{CurrentUserCtx, Field, Route, Validated, use_field, use_form_validity},
    api::{auth::create_user, page_title},
    db::{Email, Name},
    dioxus::prelude::*,
};

#[component]
pub fn CreateAccount() -> Element {
    let mut user = use_context::<CurrentUserCtx>();
    let nav = navigator();

    let validity = use_form_validity();

    let name = use_field::<Name>();
    let email = use_field::<Email>();
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        error.set(None);

        // Guard: the button is disabled unless every field parses.
        let (Ok(name), Ok(email)) = (name.value(), email.value()) else {
            return;
        };

        submitting.set(true);

        match create_user(name, email).await {
            Ok(current) => {
                user.set(current);
                nav.push(Route::Account {});
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
                Validated {
                    field: name,
                    render: move |mut f: Field<Name>| rsx! {
                        input {
                            r#type: "text",
                            value: f.text(),
                            oninput: move |e| f.set(e.value()),
                        }
                    },
                }
            }

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
                        "Creating..."
                    } else {
                        "Create account"
                    }
                }
            }
        }
    }
}
