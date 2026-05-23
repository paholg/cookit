CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO users (id, name, is_admin) VALUES (1, 'admin', 1);

CREATE TABLE ingredients (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    density_g_per_ml REAL,
    grocery_section TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE recipes (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE recipe_steps (
    id INTEGER PRIMARY KEY,
    recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    instruction TEXT NOT NULL,
    UNIQUE (recipe_id, position)
);

CREATE TABLE recipe_step_ingredients (
    id INTEGER PRIMARY KEY,
    step_id INTEGER NOT NULL REFERENCES recipe_steps(id) ON DELETE CASCADE,
    ingredient_id INTEGER NOT NULL REFERENCES ingredients(id),
    quantity REAL NOT NULL,
    unit_kind TEXT NOT NULL CHECK (unit_kind IN ('mass', 'volume', 'count', 'custom')),
    unit TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE meals (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE meal_recipes (
    id INTEGER PRIMARY KEY,
    meal_id INTEGER NOT NULL REFERENCES meals(id) ON DELETE CASCADE,
    recipe_id INTEGER NOT NULL REFERENCES recipes(id),
    multiplier REAL NOT NULL DEFAULT 1.0,
    position INTEGER NOT NULL
);

CREATE TABLE shopping_lists (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE shopping_list_items (
    id INTEGER PRIMARY KEY,
    shopping_list_id INTEGER NOT NULL REFERENCES shopping_lists(id) ON DELETE CASCADE,
    ingredient_id INTEGER NOT NULL REFERENCES ingredients(id),
    quantity REAL NOT NULL,
    unit_kind TEXT NOT NULL CHECK (unit_kind IN ('mass', 'volume', 'count', 'custom')),
    unit TEXT NOT NULL,
    checked INTEGER NOT NULL DEFAULT 0
);
