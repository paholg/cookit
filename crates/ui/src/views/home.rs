use {
    crate::{CurrentUserCtx, Route},
    api::{APP_NAME, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn Home() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let nav = navigator();

    let has_book = user.read().book.is_some();

    use_effect(move || {
        if has_book {
            nav.replace(Route::RecipeList {});
        }
    });

    if has_book {
        return rsx! {};
    }

    let logged_in = user.read().is_logged_in();

    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        header { class: "page-header",
            h1 { "{APP_NAME}" }
        }

        if logged_in {
            // A freshly-provisioned user redirected back here after creating a
            // passkey: signed in, but with no book yet.
            p { class: "empty",
                "Signed in as {user.read().user.as_ref().map(|u| u.name.to_string()).unwrap_or_default()} — no book yet."
            }
        } else {
            Link { to: Route::CreateAccount {}, class: "button primary", "Create account" }
        }
    }
}
