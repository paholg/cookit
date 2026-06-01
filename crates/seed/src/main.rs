use api::{
    db::{
        models::{
            book::BookNew,
            user::UserNew,
            user_role::{Role, UserRoleNew},
        },
        prelude::*,
        schema::{books, user_roles, users},
    },
    id::{BookId, UserId},
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut conn = api::db::conn::get_conn().await?;

    let user_id: UserId = UserNew {
        email: "paho@paholg.com",
        name: "Admin User",
    }
    .insert_into(users::table)
    .returning(users::id)
    .get_result(&mut conn)
    .await?;

    let book_id: BookId = BookNew {
        name: "Example Book",
        slug: "example",
        owner_id: user_id,
    }
    .insert_into(books::table)
    .returning(books::id)
    .get_result(&mut conn)
    .await?;

    UserRoleNew {
        book_id,
        user_id,
        role: Role::Admin,
    }
    .insert_into(user_roles::table)
    .execute(&mut conn)
    .await?;

    Ok(())
}
