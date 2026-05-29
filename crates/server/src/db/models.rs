use diesel::{AsChangeset, HasQuery, Identifiable, Insertable, pg::Pg, prelude::Associations};
use jiff_diesel::Timestamp;
use types::id::{
    BookId, IngredientId, MealId, RecipeId, RecipeStepId, RecipeStepIngredientId, ShoppingListId,
    ShoppingListItemId, UserId, UserRoleId,
};
use uuid::Uuid;

use crate::db::schema::{
    books, ingredients, meal_recipes, meals, recipe_step_ingredients, recipe_steps, recipes,
    shopping_list_items, shopping_lists, user_roles, users,
};

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: UserId,
    pub updated_at: Timestamp,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(User, foreign_key = owner_id))]
pub struct Book {
    pub id: BookId,
    pub updated_at: Timestamp,
    pub name: String,
    pub slug: String,
    pub owner_id: UserId,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = books)]
pub struct NewBook<'a> {
    pub name: &'a str,
    pub slug: &'a str,
    pub owner_id: UserId,
}

#[derive(Copy, Clone, Debug, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::db::schema::sql_types::Role"]
pub enum Role {
    Admin,
    User,
    Readonly,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(User))]
pub struct UserRole {
    pub id: UserRoleId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub user_id: UserId,
    pub role: Role,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_roles)]
pub struct NewUserRole {
    pub book_id: BookId,
    pub user_id: UserId,
    pub role: Role,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
pub struct Ingredient {
    pub id: IngredientId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub name: String,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = ingredients)]
pub struct NewIngredient<'a> {
    pub book_id: BookId,
    pub name: &'a str,
    pub density_g_per_ml: Option<f64>,
    pub grocery_section: Option<&'a str>,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
pub struct Recipe {
    pub id: RecipeId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub slug: String,
    pub name: String,
    pub source: String,
    pub description: String,
    pub notes: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recipes)]
pub struct NewRecipe<'a> {
    pub book_id: BookId,
    pub slug: &'a str,
    pub name: &'a str,
    pub source: Option<&'a str>,
    pub description: Option<&'a str>,
    pub notes: Option<&'a str>,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(Recipe))]
pub struct RecipeStep {
    pub id: RecipeStepId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub recipe_id: RecipeId,
    pub position: i32,
    pub text: String,
    pub duration_s: Option<i32>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recipe_steps)]
pub struct NewRecipeStep<'a> {
    pub book_id: BookId,
    pub recipe_id: RecipeId,
    pub position: i32,
    pub text: &'a str,
    pub duration_s: Option<i32>,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(RecipeStep, foreign_key = step_id))]
#[diesel(belongs_to(Ingredient))]
pub struct RecipeStepIngredient {
    pub id: RecipeStepIngredientId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub step_id: RecipeStepId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<String>,
    pub unit: Option<String>,
    pub ingredient_id: IngredientId,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = recipe_step_ingredients)]
pub struct NewRecipeStepIngredient<'a> {
    pub book_id: BookId,
    pub step_id: RecipeStepId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<&'a str>,
    pub unit: Option<&'a str>,
    pub ingredient_id: Uuid,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
pub struct Meal {
    pub id: MealId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = meals)]
pub struct NewMeal<'a> {
    pub book_id: BookId,
    pub slug: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(Meal))]
#[diesel(belongs_to(Recipe))]
pub struct MealRecipe {
    pub id: MealId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub meal_id: MealId,
    pub recipe_id: RecipeId,
    pub multiplier: f64,
    pub position: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = meal_recipes)]
pub struct NewMealRecipe {
    pub book_id: BookId,
    pub meal_id: MealId,
    pub recipe_id: RecipeId,
    pub multiplier: f64,
    pub position: i32,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
pub struct ShoppingList {
    pub id: ShoppingListId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = shopping_lists)]
pub struct NewShoppingList<'a> {
    pub book_id: BookId,
    pub slug: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Clone, HasQuery, Identifiable, AsChangeset, Associations)]
#[diesel(check_for_backend(Pg))]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(ShoppingList))]
#[diesel(belongs_to(Ingredient))]
pub struct ShoppingListItem {
    pub id: ShoppingListItemId,
    pub book_id: BookId,
    pub updated_at: Timestamp,
    pub shopping_list_id: ShoppingListId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<String>,
    pub unit: Option<String>,
    pub ingredient_id: Option<IngredientId>,
    pub text: Option<String>,
    pub checked: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = shopping_list_items)]
pub struct NewShoppingListItem<'a> {
    pub book_id: BookId,
    pub shopping_list_id: ShoppingListId,
    pub position: i32,
    pub quantity: Option<f64>,
    pub unit_kind: Option<&'a str>,
    pub unit: Option<&'a str>,
    pub ingredient_id: Option<IngredientId>,
    pub text: Option<&'a str>,
}
