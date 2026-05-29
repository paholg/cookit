CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE
);

SELECT
    diesel_manage_updated_at('users');

CREATE TABLE books (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    owner_id UUID NOT NULL REFERENCES users(id)
);

SELECT
    diesel_manage_updated_at('books');

CREATE TYPE role AS enum ('admin', 'user', 'readonly');

CREATE TABLE user_roles(
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID NOT NULL REFERENCES users(id),
    role role NOT NULL,
    UNIQUE(book_id, user_id)
);

SELECT
    diesel_manage_updated_at('user_roles');

CREATE TABLE ingredients (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL UNIQUE,
    density_g_per_ml DOUBLE PRECISION,
    grocery_section TEXT
);

SELECT
    diesel_manage_updated_at('ingredients');

CREATE TABLE recipes (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    description TEXT NOT NULL,
    notes TEXT NOT NULL,
    UNIQUE (book_id, slug)
);

SELECT
    diesel_manage_updated_at('recipes');

CREATE TABLE recipe_steps (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    position INTEGER NOT NULL,
    text TEXT NOT NULL,
    duration_s INTEGER,
    UNIQUE (recipe_id, position)
);

SELECT
    diesel_manage_updated_at('recipe_steps');

CREATE TABLE recipe_step_ingredients (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    step_id UUID NOT NULL REFERENCES recipe_steps(id),
    position INTEGER NOT NULL,
    quantity DOUBLE PRECISION,
    unit_kind TEXT,
    unit TEXT,
    ingredient_id UUID NOT NULL REFERENCES ingredients(id),
    UNIQUE (step_id, position)
);

SELECT
    diesel_manage_updated_at('recipe_step_ingredients');

CREATE TABLE meals (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (book_id, slug)
);

SELECT
    diesel_manage_updated_at('meals');

CREATE TABLE meal_recipes (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    meal_id UUID NOT NULL REFERENCES meals(id),
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    multiplier DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    position INTEGER NOT NULL,
    UNIQUE (meal_id, position)
);

SELECT
    diesel_manage_updated_at('meal_recipes');

CREATE TABLE shopping_lists (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (book_id, slug)
);

SELECT
    diesel_manage_updated_at('shopping_lists');

CREATE TABLE shopping_list_items (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    book_id UUID NOT NULL REFERENCES books(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    shopping_list_id UUID NOT NULL REFERENCES shopping_lists(id),
    position INTEGER NOT NULL,
    quantity DOUBLE PRECISION,
    unit_kind TEXT,
    unit TEXT,
    ingredient_id UUID REFERENCES ingredients(id),
    text TEXT,
    checked BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (shopping_list_id, position)
);

SELECT
    diesel_manage_updated_at('shopping_list_items');
