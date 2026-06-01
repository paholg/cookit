use {
    crate::{
        db::{
            conn::{DbConn, get_conn},
            models::{
                book::Book,
                user::User,
                user_role::{Role, UserRole},
            },
            schema::{books, user_roles, users},
        },
        id::{BookId, UserId},
    },
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    diesel::{BelongingToDsl, ExpressionMethods, QueryDsl},
    diesel_async::RunQueryDsl,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: UserId,
    pub book_id: BookId,
    pub name: String,
    pub email: String,
    pub role: Role,
}

#[cfg(feature = "server")]
pub struct Session {
    conn: DbConn,
    book: Book,
    user: User,
    role: UserRole,
}

#[cfg(feature = "server")]
impl Session {
    // TODO: Don't just query first.
    pub async fn create() -> anyhow::Result<Self> {
        let mut conn = get_conn().await?;

        let user: User = users::table
            .order_by(users::id.asc())
            .first(&mut conn)
            .await?;

        let book: Book = books::table
            .order_by(books::id.asc())
            .first(&mut conn)
            .await?;

        let role: UserRole = UserRole::belonging_to(&user)
            .filter(user_roles::book_id.eq(book.id))
            .first(&mut conn)
            .await?;

        Ok(Self {
            conn,
            book,
            user,
            role,
        })
    }

    pub fn book_id(&self) -> BookId {
        self.book.id
    }

    pub fn conn(&mut self) -> &mut DbConn {
        &mut self.conn
    }

    pub fn current_user(&self) -> CurrentUser {
        CurrentUser {
            id: self.user.id,
            book_id: self.book.id,
            name: self.user.name.clone(),
            email: self.user.email.clone(),
            role: self.role.role,
        }
    }
}
