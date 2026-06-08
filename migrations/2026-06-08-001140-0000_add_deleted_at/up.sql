ALTER TABLE
    users
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    books
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    user_roles
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    ingredients
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    recipes
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    recipe_steps
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    recipe_step_ingredients
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    meals
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    meal_recipes
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    shopping_lists
ADD
    COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE
    shopping_list_items
ADD
    COLUMN deleted_at TIMESTAMPTZ;

CREATE INDEX user_roles_book_id_updated_at_idx ON user_roles (book_id, updated_at);

CREATE INDEX ingredients_book_id_updated_at_idx ON ingredients (book_id, updated_at);

CREATE INDEX recipes_book_id_updated_at_idx ON recipes (book_id, updated_at);

CREATE INDEX recipe_steps_book_id_updated_at_idx ON recipe_steps (book_id, updated_at);

CREATE INDEX recipe_step_ingredients_book_id_updated_at_idx ON recipe_step_ingredients (book_id, updated_at);

CREATE INDEX meals_book_id_updated_at_idx ON meals (book_id, updated_at);

CREATE INDEX meal_recipes_book_id_updated_at_idx ON meal_recipes (book_id, updated_at);

CREATE INDEX shopping_lists_book_id_updated_at_idx ON shopping_lists (book_id, updated_at);

CREATE INDEX shopping_list_items_book_id_updated_at_idx ON shopping_list_items (book_id, updated_at);
