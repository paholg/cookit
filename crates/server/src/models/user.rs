use {
    crate::Result,
    db::{
        Email, Name,
        id::UserId,
        models::user::{User, UserCreate},
        prelude::*,
        schema::users,
    },
    diesel_async::AsyncPgConnection,
};

pub async fn create(mut conn: &AsyncPgConnection, name: Name, email: Email) -> Result<User> {
    let user = UserCreate { email, name }
        .insert_into(users::table)
        .returning(User::as_returning())
        .get_result(&mut conn)
        .await?;

    Ok(user)
}

pub async fn find(mut conn: &AsyncPgConnection, user_id: UserId) -> Result<User> {
    let user = users::table
        .filter(users::id.eq(user_id))
        .filter(users::deleted_at.is_null())
        .first(&mut conn)
        .await?;

    Ok(user)
}

pub async fn find_by_email(mut conn: &AsyncPgConnection, email: &Email) -> Result<User> {
    let user = users::table
        .filter(users::email.eq(email))
        .filter(users::deleted_at.is_null())
        .first(&mut conn)
        .await?;

    Ok(user)
}
