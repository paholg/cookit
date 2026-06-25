use {
    crate::conn::DbConn,
    db::{
        Name, Slug,
        id::UserId,
        models::{
            book::{Book, BookCreate},
            user_role::{Role, UserRoleCreate},
        },
        prelude::*,
        schema::{books, user_roles},
    },
    diesel_async::AsyncConnection,
};

pub async fn list(conn: &mut DbConn, user_id: UserId) -> crate::Result<Vec<Book>> {
    let books = user_roles::table
        .inner_join(books::table)
        .filter(user_roles::user_id.eq(user_id))
        .filter(user_roles::deleted_at.is_null())
        .filter(books::deleted_at.is_null())
        .order_by(books::name.asc())
        .select(Book::as_select())
        .load(conn)
        .await?;

    Ok(books)
}

pub async fn create(
    conn: &mut DbConn,
    owner_id: UserId,
    name: Name,
    slug: Slug,
) -> crate::Result<Book> {
    let book = conn
        .transaction(async move |conn| {
            let book: Book = BookCreate {
                name,
                slug,
                owner_id,
            }
            .insert_into(books::table)
            .returning(Book::as_returning())
            .get_result(conn)
            .await?;

            UserRoleCreate {
                book_id: book.id,
                user_id: owner_id,
                role: Role::Admin,
            }
            .insert_into(user_roles::table)
            .execute(conn)
            .await?;

            diesel::result::QueryResult::Ok(book)
        })
        .await?;

    Ok(book)
}

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
