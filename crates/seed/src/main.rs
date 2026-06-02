//! Database seeding.
//!
//! - `seed` (no args): seed the dev database with one admin user + book, plus a
//!   starter set of ingredients, recipes, and meals.
//! - `seed e2e-setup`: wipe everything, seed one admin user + book, and print
//!   `USER_ROLE_ID=<id>` for the e2e suite to log in as. No sample content, so
//!   the e2e suite starts from a known-empty book.
//! - `seed e2e-teardown`: wipe everything, leaving an empty (migrated) database.

use api::{
    Ingredient, IngredientUpdate, MealBuilder, MealRecipeBuilder, RecipeBuilder, RecipeStepBuilder,
    RecipeStepIngredientBuilder,
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
    grocery_section::GrocerySection,
    helpers::{Name, PositiveFloat},
    id::{BookId, DraftId, UserId, UserRoleId},
    session::Session,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut conn = get_conn().await?;

    match std::env::args().nth(1).as_deref() {
        None => {
            seed(&mut conn).await?;

            // Sample content goes through the same builders the app uses, so it
            // needs a `Session`. Outside any HTTP request it resolves to the
            // first user/book — exactly the admin/book we just seeded.
            let mut session = Session::from_request()
                .await
                .map_err(|e| eyre::eyre!("load session: {e}"))?
                .ok_or_else(|| eyre::eyre!("no session after seeding admin user/book"))?;
            seed_content(&mut session).await?;
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

/// Seed a starter cookbook: a handful of recipes (which create their referenced
/// ingredients), a few meals composed of those recipes, and density/grocery
/// section details filled in on the common ingredients.
async fn seed_content(session: &mut Session) -> eyre::Result<()> {
    let recipes = vec![
        recipe(
            "Spaghetti Bolognese",
            "https://example.com/bolognese",
            vec![
                step(
                    "Brown the ground beef in a large pot, breaking it up as it cooks.",
                    "10m",
                    vec![ing("Ground Beef", "1", "lb"), ing("Olive Oil", "2", "tbsp")],
                ),
                step(
                    "Add the onion and garlic and cook until soft.",
                    "5m",
                    vec![ing("Onion", "1", "whole"), ing("Garlic", "3", "clove")],
                ),
                step(
                    "Stir in the tomatoes and salt, then simmer.",
                    "30m",
                    vec![ing("Crushed Tomatoes", "28", "oz"), ing("Salt", "1", "tsp")],
                ),
                step(
                    "Boil the spaghetti until al dente, drain, and serve under the sauce.",
                    "12m",
                    vec![ing("Spaghetti", "1", "lb")],
                ),
            ],
        ),
        recipe(
            "Caesar Salad",
            "https://example.com/caesar",
            vec![
                step(
                    "Chop the romaine and add it to a large bowl.",
                    "",
                    vec![ing("Romaine Lettuce", "2", "head")],
                ),
                step(
                    "Whisk the olive oil and garlic into a dressing and toss with the lettuce.",
                    "",
                    vec![ing("Olive Oil", "3", "tbsp"), ing("Garlic", "1", "clove")],
                ),
                step(
                    "Top with parmesan and croutons.",
                    "",
                    vec![ing("Parmesan", "0.5", "cup"), ing("Croutons", "1", "cup")],
                ),
            ],
        ),
        recipe(
            "Classic Pancakes",
            "https://example.com/pancakes",
            vec![
                step(
                    "Whisk together the flour, sugar, and salt.",
                    "",
                    vec![
                        ing("All-Purpose Flour", "2", "cup"),
                        ing("Sugar", "2", "tbsp"),
                        ing("Salt", "0.5", "tsp"),
                    ],
                ),
                step(
                    "Beat in the milk, eggs, and melted butter until just combined.",
                    "",
                    vec![
                        ing("Milk", "1.5", "cup"),
                        ing("Eggs", "2", "whole"),
                        ing("Butter", "3", "tbsp"),
                    ],
                ),
                step(
                    "Cook on a hot griddle until bubbles form, then flip.",
                    "4m",
                    vec![],
                ),
            ],
        ),
        recipe(
            "Guacamole",
            "https://example.com/guacamole",
            vec![
                step(
                    "Mash the avocados in a bowl.",
                    "",
                    vec![ing("Avocado", "3", "whole")],
                ),
                step(
                    "Stir in the lime juice, onion, cilantro, and salt.",
                    "",
                    vec![
                        ing("Lime", "1", "whole"),
                        ing("Onion", "0.5", "whole"),
                        ing("Cilantro", "0.25", "cup"),
                        ing("Salt", "0.5", "tsp"),
                    ],
                ),
            ],
        ),
        recipe(
            "Chicken Stir-Fry",
            "https://example.com/stir-fry",
            vec![
                step(
                    "Cook the rice according to package directions.",
                    "20m",
                    vec![ing("Rice", "1", "cup")],
                ),
                step(
                    "Sear the sliced chicken in oil over high heat.",
                    "6m",
                    vec![ing("Chicken Breast", "1", "lb"), ing("Olive Oil", "2", "tbsp")],
                ),
                step(
                    "Add the vegetables and garlic and stir-fry until crisp-tender.",
                    "5m",
                    vec![
                        ing("Bell Pepper", "2", "whole"),
                        ing("Broccoli", "2", "cup"),
                        ing("Garlic", "2", "clove"),
                    ],
                ),
                step(
                    "Pour in the soy sauce, toss, and serve over rice.",
                    "",
                    vec![ing("Soy Sauce", "3", "tbsp")],
                ),
            ],
        ),
    ];

    // Persisting a recipe creates any ingredients it names, so keep the slugs to
    // wire up meals afterward.
    let mut slugs = std::collections::HashMap::new();
    for builder in recipes {
        let name = builder.name.clone();
        let detail = builder
            .upsert(session)
            .await
            .map_err(|e| eyre::eyre!("seed recipe {name:?}: {e}"))?;
        slugs.insert(name, detail.recipe.slug);
    }

    let meals = vec![
        meal(
            "Italian Dinner",
            vec![
                (&slugs["Spaghetti Bolognese"], "1"),
                (&slugs["Caesar Salad"], "1"),
            ],
        ),
        meal("Weekend Brunch", vec![(&slugs["Classic Pancakes"], "2")]),
        meal(
            "Quick Weeknight",
            vec![
                (&slugs["Chicken Stir-Fry"], "1"),
                (&slugs["Guacamole"], "0.5"),
            ],
        ),
    ];

    for builder in meals {
        let name = builder.name.clone();
        builder
            .upsert(session)
            .await
            .map_err(|e| eyre::eyre!("seed meal {name:?}: {e}"))?;
    }

    enrich_ingredients(session).await?;

    Ok(())
}

/// Fill in density and grocery section on the ingredients the recipes created.
/// Recipes only create bare ingredients (just a name); this is the data the
/// ingredient editor would otherwise add by hand.
async fn enrich_ingredients(session: &mut Session) -> eyre::Result<()> {
    use GrocerySection::*;

    // (name, density g/ml if it's a liquid/powder worth converting, section)
    let details: &[(&str, Option<f64>, GrocerySection)] = &[
        ("Olive Oil", Some(0.91), Pantry),
        ("All-Purpose Flour", Some(0.53), Pantry),
        ("Sugar", Some(0.85), Pantry),
        ("Salt", Some(1.2), Pantry),
        ("Milk", Some(1.03), Dairy),
        ("Soy Sauce", Some(1.1), Pantry),
        ("Ground Beef", None, Meat),
        ("Chicken Breast", None, Meat),
        ("Onion", None, Produce),
        ("Garlic", None, Produce),
        ("Avocado", None, Produce),
        ("Lime", None, Produce),
        ("Cilantro", None, Produce),
        ("Bell Pepper", None, Produce),
        ("Broccoli", None, Produce),
        ("Romaine Lettuce", None, Produce),
        ("Crushed Tomatoes", None, Pantry),
        ("Spaghetti", None, Pantry),
        ("Rice", None, Pantry),
        ("Parmesan", None, Dairy),
        ("Eggs", None, Dairy),
        ("Butter", None, Dairy),
        ("Croutons", None, Bakery),
    ];

    // One lookup of everything in the book, then match by name.
    let by_name: std::collections::HashMap<String, Ingredient> = Ingredient::list_all(session)
        .await
        .map_err(|e| eyre::eyre!("list ingredients: {e}"))?
        .into_iter()
        .map(|i| (i.name.as_ref().to_string(), i))
        .collect();

    for &(name, density, section) in details {
        let Some(ingredient) = by_name.get(name) else {
            // A detail row that no recipe referenced — skip rather than fail.
            continue;
        };

        let density_g_per_ml = match density {
            Some(d) => Some(PositiveFloat::parse(d).map_err(|e| eyre::eyre!("{name} density: {e}"))?),
            None => None,
        };

        let update = IngredientUpdate {
            name: Name::parse(name).map_err(|e| eyre::eyre!("{name}: {e}"))?,
            density_g_per_ml,
            grocery_section: Some(section),
        };

        update
            .apply(ingredient.id, session)
            .await
            .map_err(|e| eyre::eyre!("update ingredient {name:?}: {e}"))?;
    }

    Ok(())
}

/// A recipe builder with a fresh draft id and the given steps.
fn recipe(name: &str, source: &str, steps: Vec<RecipeStepBuilder>) -> RecipeBuilder {
    RecipeBuilder {
        id: DraftId::New(0),
        name: name.to_string(),
        source: source.to_string(),
        steps,
    }
}

/// A recipe step with an instruction, optional `duration_text` timer (empty for
/// none), and ingredient rows.
fn step(
    instruction: &str,
    duration: &str,
    ingredients: Vec<RecipeStepIngredientBuilder>,
) -> RecipeStepBuilder {
    RecipeStepBuilder {
        id: DraftId::New(0),
        instruction: instruction.to_string(),
        duration_text: duration.to_string(),
        ingredients,
    }
}

/// One ingredient row within a step.
fn ing(name: &str, quantity: &str, unit: &str) -> RecipeStepIngredientBuilder {
    RecipeStepIngredientBuilder {
        id: DraftId::New(0),
        name: name.to_string(),
        quantity: quantity.to_string(),
        unit: unit.to_string(),
    }
}

/// A meal builder referencing recipes by slug, each with a multiplier.
fn meal(name: &str, recipes: Vec<(&str, &str)>) -> MealBuilder {
    let recipes = recipes
        .into_iter()
        .map(|(slug, multiplier)| MealRecipeBuilder {
            id: DraftId::New(0),
            recipe_slug: slug.to_string(),
            multiplier: multiplier.to_string(),
        })
        .collect();

    MealBuilder {
        id: DraftId::New(0),
        name: name.to_string(),
        recipes,
    }
}
