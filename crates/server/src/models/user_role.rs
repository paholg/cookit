use {
    crate::conn::DbConn,
    db::{
        id::{BookId, UserId},
        models::user_role::UserRole,
        prelude::*,
        schema::user_roles,
    },
};

pub async fn try_find(
    conn: &mut DbConn,
    user_id: UserId,
    book_id: BookId,
) -> crate::Result<Option<UserRole>> {
    let role = user_roles::table
        .filter(user_roles::user_id.eq(user_id))
        .filter(user_roles::book_id.eq(book_id))
        .filter(user_roles::deleted_at.is_null())
        .first(conn)
        .await
        .optional()?;

    Ok(role)
}
