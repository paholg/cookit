-- Dev-only seed data. Safe to re-run; every INSERT is idempotent.
-- Invoked by `just seed` and by the devcontainer's postCreateCommand.
-- Never run against a production database.
 BEGIN TRANSACTION;

-- ---------------------------------------------------------------------------
-- Dev users. oidc_sub values match the dev-auth login handlers.
-- ---------------------------------------------------------------------------

INSERT INTO users (oidc_sub, email, name, groups, is_admin)
VALUES ('123', 'user@local', 'User', '', 0),
       ('456', 'admin@local', 'Admin', '', 1) ON conflict(oidc_sub) DO
UPDATE
SET email = excluded.email,
    name = excluded.name,
    groups = excluded.groups,
    is_admin = excluded.is_admin;

-- ---------------------------------------------------------------------------
-- Ingredients
-- ---------------------------------------------------------------------------

INSERT INTO ingredients (name, density_g_per_ml, grocery_section, ignore_density)
VALUES ('flour', 0.53, 'bakery', 0),
       ('sugar', 0.85, 'bakery', 0),
       ('salt', 1.20, 'bakery', 0),
       ('butter', 0.91, 'dairy', 0),
       ('eggs', NULL, 'dairy', 1),
       ('olive oil', 0.91, 'pantry', 0),
       ('tomato', NULL, 'produce', 1),
       ('garlic', NULL, 'produce', 1),
       ('onion', NULL, 'produce', 1),
       ('pasta', NULL, 'pantry', 0) ON conflict(name) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Recipes
-- ---------------------------------------------------------------------------
 -- Recipe 1: Simple Pasta -----------------------------------------------------

INSERT INTO recipes (name, SOURCE)
SELECT 'Simple Pasta',
       'seed'
WHERE NOT EXISTS
    (SELECT 1
     FROM recipes
     WHERE name = 'Simple Pasta'
       AND SOURCE = 'seed');


INSERT INTO recipe_steps (recipe_id, POSITION)
SELECT r.id,
       p.position
FROM recipes r
CROSS JOIN
  (SELECT 0 AS POSITION
   UNION ALL SELECT 1) p
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_steps
     WHERE recipe_id = r.id);


INSERT INTO recipe_step_instructions (step_id, POSITION, text)
SELECT s.id,
       0,
       'Bring a large pot of salted water to a boil.'
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_instructions
     WHERE step_id = s.id);


INSERT INTO recipe_step_instructions (step_id, POSITION, text)
SELECT s.id,
       0,
       'Add the pasta and cook until al dente, then drain and toss with olive oil.'
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND s.position = 1
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_instructions
     WHERE step_id = s.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       10,
       'mass',
       'g',
       0
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'salt'
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       200,
       'mass',
       'g',
       0
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'pasta'
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND s.position = 1
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       15,
       'volume',
       'ml',
       1
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'olive oil'
WHERE r.name = 'Simple Pasta'
  AND r.source = 'seed'
  AND s.position = 1
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id
       AND ingredient_id = i.id);

-- Recipe 2: Pancakes ---------------------------------------------------------

INSERT INTO recipes (name, SOURCE)
SELECT 'Pancakes',
       'seed'
WHERE NOT EXISTS
    (SELECT 1
     FROM recipes
     WHERE name = 'Pancakes'
       AND SOURCE = 'seed');


INSERT INTO recipe_steps (recipe_id, POSITION)
SELECT r.id,
       p.position
FROM recipes r
CROSS JOIN
  (SELECT 0 AS POSITION
   UNION ALL SELECT 1) p
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_steps
     WHERE recipe_id = r.id);


INSERT INTO recipe_step_instructions (step_id, POSITION, text)
SELECT s.id,
       0,
       'Whisk the flour, sugar, salt, and eggs into a smooth batter.'
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_instructions
     WHERE step_id = s.id);


INSERT INTO recipe_step_instructions (step_id, POSITION, text)
SELECT s.id,
       0,
       'Melt butter on a hot griddle and cook the pancakes until golden on both sides.'
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 1
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_instructions
     WHERE step_id = s.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       200,
       'mass',
       'g',
       0
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'flour'
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       30,
       'mass',
       'g',
       1
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'sugar'
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id
       AND ingredient_id = i.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       2,
       'mass',
       'g',
       2
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'salt'
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id
       AND ingredient_id = i.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       2,
       'count',
       'whole',
       3
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'eggs'
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 0
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id
       AND ingredient_id = i.id);


INSERT INTO recipe_step_ingredients (step_id, ingredient_id, quantity, unit_kind, unit, POSITION)
SELECT s.id,
       i.id,
       15,
       'mass',
       'g',
       0
FROM recipe_steps s
JOIN recipes r ON r.id = s.recipe_id
JOIN ingredients i ON i.name = 'butter'
WHERE r.name = 'Pancakes'
  AND r.source = 'seed'
  AND s.position = 1
  AND NOT EXISTS
    (SELECT 1
     FROM recipe_step_ingredients
     WHERE step_id = s.id);


COMMIT;