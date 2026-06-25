use {
    crate::{BASE_DOMAIN, Field, Validated, client::client, use_field, use_form_validity},
    api::{create_book, page_title},
    db::{Name, Slug},
    dioxus::prelude::*,
};

#[component]
pub fn CreateBook() -> Element {
    let validity = use_form_validity();

    let name = use_field::<Name>();
    let slug = use_field::<Slug>();
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |e: FormEvent| async move {
        e.prevent_default();
        error.set(None);

        // Guard: the button is disabled unless every field parses.
        let (Ok(name), Ok(slug)) = (name.value(), slug.value()) else {
            return;
        };

        submitting.set(true);

        match create_book(name, slug).await {
            Ok(book) => client().set_current_book(Some(&book)),
            Err(e) => {
                error.set(Some(e.to_string()));
                submitting.set(false);
            }
        }
    };

    rsx! {
        document::Title { "{page_title(\"Create cookbook\")}" }
        header { class: "page-header",
            h1 { "Create cookbook" }
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
                "Url"
                Validated {
                    field: slug,
                    hint: "Lowercase letters, numbers, dashes and underscores; at least 4 characters.",
                    render: move |mut f: Field<Slug>| rsx! {
                        span { class: "slug-field",
                            input {
                                r#type: "text",
                                value: f.text(),
                                oninput: move |e| f.set(e.value()),
                            }
                            span { class: "slug-suffix", ".{BASE_DOMAIN}" }
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
                        "Create cookbook"
                    }
                }
            }
        }
    }
}
