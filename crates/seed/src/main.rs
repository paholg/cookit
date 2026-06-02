//! Database seeding.
//!
//! - `seed` (no args): seed the dev database with one admin user + book.
//! - `seed e2e-setup`: wipe everything, seed one admin user + book, and print
//!   `USER_ROLE_ID=<id>` for the e2e suite to log in as.
//! - `seed e2e-teardown`: wipe everything, leaving an empty (migrated) database.

use api::{
    db::{
        conn::{DbConn, get_conn},
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

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut conn = get_conn().await?;

    match std::env::args().nth(1).as_deref() {
        None => {
            seed(&mut conn).await?;
        }
        Some("e2e-setup") => {
            wipe(&mut conn).await?;
            let (user_role_id, book_slug) = seed(&mut conn).await?;
            // Parsed by the e2e global setup.
            println!("USER_ROLE_ID={user_role_id}");
            println!("BOOK_SLUG={book_slug}");
        }
        Some("e2e-teardown") => {
            wipe(&mut conn).await?;
        }
        Some(other) => {
            eyre::bail!("unknown seed command: {other:?} (expected e2e-setup or e2e-teardown)");
        }
    }

    Ok(())
}

/// Insert one admin user and a book they own. Returns the new `user_role` id and
/// the book slug.
async fn seed(conn: &mut DbConn) -> eyre::Result<(UserRoleId, String)> {
    let user_id: UserId = UserNew {
        email: "paho@paholg.com",
        name: "Admin User",
    }
    .insert_into(users::table)
    .returning(users::id)
    .get_result(conn)
    .await?;

    let book_slug = "example";
    let book_id: BookId = BookNew {
        name: "Example Book",
        slug: book_slug,
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

    Ok((user_role_id, book_slug.to_string()))
}

/// Empty every domain table. `CASCADE` clears child rows; the parent list covers
/// all tables so nothing survives.
async fn wipe(conn: &mut DbConn) -> eyre::Result<()> {
    diesel::sql_query(
        "TRUNCATE users, books, user_roles, recipes, recipe_steps, recipe_step_ingredients, \
         ingredients, meals, meal_recipes, shopping_lists, shopping_list_items RESTART IDENTITY \
         CASCADE",
    )
    .execute(conn)
    .await?;

    Ok(())
}
