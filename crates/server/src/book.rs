use {
    crate::conn::DbConn,
    db::{
        Slug,
        id::UserId,
        models::book::Book,
        prelude::*,
        schema::{books, user_roles},
    },
};

pub async fn find_by_slug(conn: &mut DbConn, slug: &Slug) -> crate::Result<Option<Book>> {
    let book = books::table
        .filter(books::slug.eq(slug))
        .filter(books::deleted_at.is_null())
        .first(conn)
        .await
        .optional()?;

    Ok(book)
}

/// The user's "home" book — their first by role — used to pick which book
/// subdomain to land them on after login. `None` if they belong to no book.
// TODO: We should prioritize an "owned" book.
pub async fn load_home_book(conn: &mut DbConn, user_id: UserId) -> crate::Result<Option<Book>> {
    let book = user_roles::table
        .inner_join(books::table)
        .filter(user_roles::user_id.eq(user_id))
        .filter(user_roles::deleted_at.is_null())
        .filter(books::deleted_at.is_null())
        .order_by(user_roles::id.asc())
        .select(Book::as_select())
        .first(conn)
        .await
        .optional()?;

    Ok(book)
}
