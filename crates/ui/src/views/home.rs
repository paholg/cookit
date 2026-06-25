use {
    crate::{CurrentUserCtx, Route, client::client},
    api::{APP_NAME, list_books, page_title},
    dioxus::prelude::*,
};

#[component]
pub fn Home() -> Element {
    let user = use_context::<CurrentUserCtx>();
    let nav = navigator();

    let has_book = user.read().book.is_some();
    let logged_in = user.read().is_logged_in();

    let books = use_server_future(list_books)?;

    use_effect(move || {
        if has_book {
            nav.replace(Route::RecipeList {});
        }
    });

    if has_book {
        return rsx! {};
    }

    rsx! {
        document::Title { "{page_title(APP_NAME)}" }
        header { class: "page-header",
            h1 { "{APP_NAME}" }
        }

        if logged_in {
            match books.cloned() {
                Some(Ok(list)) if !list.is_empty() => rsx! {
                    p { "Open a cookbook:" }
                    div { class: "book-list",
                        for book in list {
                            button {
                                class: "button primary",
                                onclick: {
                                    let book = book.clone();
                                    move |_| client().set_current_book(Some(&book))
                                },
                                "{book.name}"
                            }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    p { "You don't have any cookbooks yet." }
                    Link { to: Route::CreateBook {}, class: "button primary", "Create cookbook" }
                },
                Some(Err(e)) => rsx! {
                    p { class: "error", "Error loading cookbooks: {e}" }
                },
                None => rsx! {
                    p { "Loading..." }
                },
            }
        } else {
            Link { to: Route::CreateAccount {}, class: "button primary", "Create account" }
        }
    }
}
