DROP INDEX IF EXISTS shopping_list_items_book_id_updated_at_idx;

DROP INDEX IF EXISTS shopping_lists_book_id_updated_at_idx;

DROP INDEX IF EXISTS meal_recipes_book_id_updated_at_idx;

DROP INDEX IF EXISTS meals_book_id_updated_at_idx;

DROP INDEX IF EXISTS recipe_step_ingredients_book_id_updated_at_idx;

DROP INDEX IF EXISTS recipe_steps_book_id_updated_at_idx;

DROP INDEX IF EXISTS recipes_book_id_updated_at_idx;

DROP INDEX IF EXISTS ingredients_book_id_updated_at_idx;

DROP INDEX IF EXISTS user_roles_book_id_updated_at_idx;

ALTER TABLE
    shopping_list_items DROP COLUMN deleted_at;

ALTER TABLE
    shopping_lists DROP COLUMN deleted_at;

ALTER TABLE
    meal_recipes DROP COLUMN deleted_at;

ALTER TABLE
    meals DROP COLUMN deleted_at;

ALTER TABLE
    recipe_step_ingredients DROP COLUMN deleted_at;

ALTER TABLE
    recipe_steps DROP COLUMN deleted_at;

ALTER TABLE
    recipes DROP COLUMN deleted_at;

ALTER TABLE
    ingredients DROP COLUMN deleted_at;

ALTER TABLE
    user_roles DROP COLUMN deleted_at;

ALTER TABLE
    books DROP COLUMN deleted_at;

ALTER TABLE
    users DROP COLUMN deleted_at;
