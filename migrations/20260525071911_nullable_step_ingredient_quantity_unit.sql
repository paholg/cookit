-- Allow recipe_step_ingredients.quantity, unit_kind, and unit to be NULL so
-- a step can list an ingredient without specifying a quantity or unit
-- (e.g. "salt to taste"). SQLite can't drop NOT NULL in place, so rebuild.

CREATE TABLE recipe_step_ingredients_new (
    id INTEGER PRIMARY KEY,
    step_id INTEGER NOT NULL REFERENCES recipe_steps(id) ON DELETE CASCADE,
    ingredient_id INTEGER NOT NULL REFERENCES ingredients(id),
    quantity REAL,
    unit_kind TEXT CHECK (unit_kind IN ('mass', 'volume', 'count', 'custom')),
    unit TEXT,
    position INTEGER NOT NULL
);

INSERT INTO recipe_step_ingredients_new
    (id, step_id, ingredient_id, quantity, unit_kind, unit, position)
SELECT id, step_id, ingredient_id, quantity, unit_kind, unit, position
FROM recipe_step_ingredients;

DROP TABLE recipe_step_ingredients;
ALTER TABLE recipe_step_ingredients_new RENAME TO recipe_step_ingredients;
