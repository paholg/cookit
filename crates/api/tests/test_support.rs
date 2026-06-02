use {
    api::{
        db::{
            conn,
            models::{
                book::BookNew,
                user::UserNew,
                user_role::{Role, UserRoleNew},
            },
            prelude::*,
            schema::{books, user_roles, users},
        },
        id::{BookId, UserId, UserRoleId},
    },
    std::time::{SystemTime, UNIX_EPOCH},
    uuid::Uuid,
};

pub fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix} {nanos}")
}

#[allow(unused)]
pub struct TestBook {
    pub user_id: UserId,
    pub book_id: BookId,
    pub user_role_id: UserRoleId,
    pub email: String,
    pub slug: String,
}

impl TestBook {
    pub async fn new() -> Self {
        let mut conn = conn::get_conn().await.unwrap();

        let token = Uuid::now_v7().simple().to_string();
        let email = format!("test-{token}@example.test");
        let slug = format!("test-book-{token}");

        let user_id: UserId = diesel::insert_into(users::table)
            .values(UserNew {
                email: &email,
                name: "Test User",
            })
            .returning(users::id)
            .get_result(&mut conn)
            .await
            .unwrap();

        let book_id: BookId = diesel::insert_into(books::table)
            .values(BookNew {
                name: "Test Book",
                slug: &slug,
                owner_id: user_id,
            })
            .returning(books::id)
            .get_result(&mut conn)
            .await
            .unwrap();

        let user_role_id: UserRoleId = diesel::insert_into(user_roles::table)
            .values(UserRoleNew {
                book_id,
                user_id,
                role: Role::Admin,
            })
            .returning(user_roles::id)
            .get_result(&mut conn)
            .await
            .unwrap();

        Self {
            user_id,
            book_id,
            user_role_id,
            email,
            slug,
        }
    }
}
