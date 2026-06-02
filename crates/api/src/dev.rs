//! Test-data helpers for the e2e suite.
//!
//! These back the unauthenticated `/api/dev/*` endpoints (see [`crate::routes`])
//! and the no-arg `seed` binary. They create and tear down an isolated
//! admin user + book so a browser run never has to shell out to `diesel` or
//! `cargo`.

use crate::{
    db::{
        conn::DbConn,
        models::{
            book::BookNew,
            user::UserNew,
            user_role::{Role, UserRoleNew},
        },
        prelude::*,
        schema::{books, user_roles, users},
    },
    id::{BookId, UserId, UserRoleId},
};

/// Insert an admin user, a book they own, and the matching `user_role`, all
/// keyed by a fresh unique suffix. Returns the three new ids.
///
/// The suffix keeps each run isolated: a crashed run that never cleaned up
/// won't collide with the next one's `UNIQUE` email/slug.
pub async fn create_test_book(conn: &mut DbConn) -> anyhow::Result<(UserId, BookId, UserRoleId)> {
    let suffix = jiff::Timestamp::now().as_nanosecond();

    let email = format!("e2e-{suffix}@example.com");
    let slug = format!("e2e-{suffix}");

    let user_id: UserId = UserNew {
        email: &email,
        name: "E2E Admin",
    }
    .insert_into(users::table)
    .returning(users::id)
    .get_result(conn)
    .await?;

    let book_id: BookId = BookNew {
        name: "E2E Book",
        slug: &slug,
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

    Ok((user_id, book_id, user_role_id))
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
