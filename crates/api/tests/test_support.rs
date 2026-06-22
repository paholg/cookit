use {
    axum::{Router, body::Body, http::Request, routing::get},
    db::{
        Email, Name, Slug,
        models::{
            book::{Book, BookNew},
            user::{User, UserCreate},
            user_role::{Role, UserRoleNew},
        },
        prelude::*,
        schema::{books, user_roles, users},
    },
    dioxus::fullstack::FullstackContext,
    server::{AuthUser, CookitAuthSession, config::config, conn, install},
    std::{
        future::Future,
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    },
    tower::ServiceExt,
    uuid::Uuid,
};

pub fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix} {nanos}")
}

pub struct TestBook {
    pub user: User,
    pub book: Book,
}

impl TestBook {
    pub async fn new() -> Self {
        let mut conn = conn::get_conn().await.unwrap();

        let token = Uuid::now_v7().simple().to_string();
        let email = Email::try_from(format!("test-{token}@example.test")).unwrap();
        let slug = Slug::try_from(format!("test-book-{token}")).unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(UserCreate {
                email: email.clone(),
                name: Name::try_from("Test User").unwrap(),
            })
            .returning(User::as_returning())
            .get_result(&mut conn)
            .await
            .unwrap();

        let book: Book = diesel::insert_into(books::table)
            .values(BookNew {
                name: Name::try_from("Test Book").unwrap(),
                slug: slug.clone(),
                owner_id: user.id,
            })
            .returning(Book::as_returning())
            .get_result(&mut conn)
            .await
            .unwrap();

        diesel::insert_into(user_roles::table)
            .values(UserRoleNew {
                book_id: book.id,
                user_id: user.id,
                role: Role::Admin,
            })
            .execute(&mut conn)
            .await
            .unwrap();

        Self { user, book }
    }

    /// Run the given future authenticated as our user, scoped to our book.
    pub async fn as_user<F: Future>(&self, fut: F) -> F::Output {
        let auth = mint_auth_session(self.user.clone()).await;

        let parts = Request::builder()
            .header("x-forwarded-host", config().host(Some(&self.book)))
            .extension(auth)
            .body(())
            .unwrap()
            .into_parts()
            .0;

        FullstackContext::new(parts).scope(fut).await
    }
}

async fn mint_auth_session(user: User) -> CookitAuthSession {
    let slot: Arc<Mutex<Option<CookitAuthSession>>> = Arc::default();
    let captured = slot.clone();

    let router = Router::new().route(
        "/",
        get(move |mut auth: CookitAuthSession| {
            let slot = captured.clone();
            async move {
                auth.current_user = Some(AuthUser { user: Some(user) });
                *slot.lock().unwrap() = Some(auth);
            }
        }),
    );
    let router = install(router).await;

    router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("mint session: oneshot");

    slot.lock().unwrap().take().expect("mint session: captured")
}
