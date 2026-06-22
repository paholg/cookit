// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "role"))]
    pub struct Role;
}

diesel::table! {
    books (id) {
        id -> Uuid,
        updated_at -> Timestamptz,
        name -> Text,
        slug -> Text,
        owner_id -> Uuid,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ingredients (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        name -> Text,
        density_g_per_ml -> Nullable<Float8>,
        grocery_section -> Nullable<Text>,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    meal_recipes (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        meal_id -> Uuid,
        recipe_id -> Uuid,
        multiplier -> Float8,
        position -> Int4,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    meals (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        slug -> Text,
        name -> Text,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    recipe_step_ingredients (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        step_id -> Uuid,
        position -> Int4,
        quantity -> Nullable<Float8>,
        unit_kind -> Nullable<Text>,
        unit -> Nullable<Text>,
        ingredient_id -> Uuid,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    recipe_steps (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        recipe_id -> Uuid,
        position -> Int4,
        text -> Text,
        duration_s -> Nullable<Int4>,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    recipes (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        slug -> Text,
        name -> Text,
        source -> Text,
        description -> Text,
        notes -> Text,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    sessions (id) {
        id -> Text,
        expires_at -> Timestamptz,
        session -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    shopping_list_items (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        shopping_list_id -> Uuid,
        position -> Int4,
        quantity -> Nullable<Float8>,
        unit_kind -> Nullable<Text>,
        unit -> Nullable<Text>,
        ingredient_id -> Nullable<Uuid>,
        text -> Nullable<Text>,
        checked -> Bool,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    shopping_lists (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        slug -> Text,
        name -> Text,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_passkey_authentications (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        passkey_authentication -> Jsonb,
    }
}

diesel::table! {
    user_passkey_registrations (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        passkey_registration -> Jsonb,
    }
}

diesel::table! {
    user_passkeys (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
        user_id -> Uuid,
        credential_id -> Text,
        passkey -> Jsonb,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Role;

    user_roles (id) {
        id -> Uuid,
        book_id -> Uuid,
        updated_at -> Timestamptz,
        user_id -> Uuid,
        role -> Role,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        updated_at -> Timestamptz,
        name -> Text,
        email -> Text,
        deleted_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(books -> users (owner_id));
diesel::joinable!(ingredients -> books (book_id));
diesel::joinable!(meal_recipes -> books (book_id));
diesel::joinable!(meal_recipes -> meals (meal_id));
diesel::joinable!(meal_recipes -> recipes (recipe_id));
diesel::joinable!(meals -> books (book_id));
diesel::joinable!(recipe_step_ingredients -> books (book_id));
diesel::joinable!(recipe_step_ingredients -> ingredients (ingredient_id));
diesel::joinable!(recipe_step_ingredients -> recipe_steps (step_id));
diesel::joinable!(recipe_steps -> books (book_id));
diesel::joinable!(recipe_steps -> recipes (recipe_id));
diesel::joinable!(recipes -> books (book_id));
diesel::joinable!(shopping_list_items -> books (book_id));
diesel::joinable!(shopping_list_items -> ingredients (ingredient_id));
diesel::joinable!(shopping_list_items -> shopping_lists (shopping_list_id));
diesel::joinable!(shopping_lists -> books (book_id));
diesel::joinable!(user_passkey_authentications -> users (user_id));
diesel::joinable!(user_passkey_registrations -> users (user_id));
diesel::joinable!(user_passkeys -> users (user_id));
diesel::joinable!(user_roles -> books (book_id));
diesel::joinable!(user_roles -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    books,
    ingredients,
    meal_recipes,
    meals,
    recipe_step_ingredients,
    recipe_steps,
    recipes,
    sessions,
    shopping_list_items,
    shopping_lists,
    user_passkey_authentications,
    user_passkey_registrations,
    user_passkeys,
    user_roles,
    users,
);
