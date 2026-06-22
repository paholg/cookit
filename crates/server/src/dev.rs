//! Test-data helpers for the e2e suite.
//!
//! These back the unauthenticated `/api/dev/*` endpoints (defined in the api
//! crate's routes) and the no-arg `seed` binary. They create and tear down an isolated
//! admin user + book so a browser run never has to shell out to `diesel` or
//! `cargo`.

use {
    crate::conn::DbConn,
    db::{
        Email, Slug,
        id::{BookId, UserId, UserRoleId},
        models::{
            book::BookNew,
            user::UserCreate,
            user_role::{Role, UserRoleNew},
        },
        schema::{books, user_roles, users},
    },
    diesel::prelude::*,
    diesel_async::RunQueryDsl,
};

/// Insert an admin user, a book they own, and the matching `user_role`, all
/// keyed by a fresh unique suffix. Returns the three new ids.
///
/// The suffix keeps each run isolated: a crashed run that never cleaned up
/// won't collide with the next one's `UNIQUE` email/slug.
pub async fn create_test_book(
    conn: &mut DbConn,
) -> anyhow::Result<(UserId, BookId, UserRoleId, Slug)> {
    let suffix = jiff::Timestamp::now().as_nanosecond();

    let email = Email::try_from(format!("e2e-{suffix}@example.com"))?;
    let slug = Slug::try_from(format!("e2e-{suffix}"))?;

    let user_id: UserId = UserCreate {
        email,
        name: "E2E Admin".try_into()?,
    }
    .insert_into(users::table)
    .returning(users::id)
    .get_result(conn)
    .await?;

    let book_id: BookId = BookNew {
        name: "E2E Book".try_into()?,
        slug: slug.clone(),
        owner_id: user_id,
    }
    .insert_into(books::table)
    .returning(books::id)
    .get_result(conn)
    .await?;

    let user_role_id: UserRoleId = UserRoleNew {
        book_id,
        user_id,
        role: Role::Admin,
    }
    .insert_into(user_roles::table)
    .returning(user_roles::id)
    .get_result(conn)
    .await?;

    Ok((user_id, book_id, user_role_id, slug))
}

/// Delete the test book and user. The book goes first so its `ON DELETE
/// CASCADE` children (the `user_role` plus all book-scoped rows) are gone before
/// we remove the user, which `books.owner_id` would otherwise still reference.
pub async fn delete_test_book(
    conn: &mut DbConn,
    user_id: UserId,
    book_id: BookId,
) -> anyhow::Result<()> {
    diesel::delete(books::table.find(book_id))
        .execute(conn)
        .await?;

    diesel::delete(users::table.find(user_id))
        .execute(conn)
        .await?;

    Ok(())
}
