-- Wipe legacy seed data (dev DBs only -- no production users yet).
DELETE FROM shopping_list_items WHERE shopping_list_id IN
  (SELECT id FROM shopping_lists WHERE user_id = 1);
DELETE FROM shopping_lists WHERE user_id = 1;
DELETE FROM meal_recipes WHERE meal_id IN (SELECT id FROM meals WHERE user_id = 1);
DELETE FROM meals WHERE user_id = 1;
DELETE FROM users WHERE id = 1;

-- Rebuild users with OIDC columns NOT NULL. SQLite can't add NOT NULL UNIQUE
-- columns to an existing table without a rebuild, so we do it explicitly.
CREATE TABLE users_new (
    id INTEGER PRIMARY KEY,
    oidc_sub TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    groups TEXT NOT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO users_new (id, oidc_sub, email, name, groups, is_admin, created_at)
SELECT id, '__legacy_' || id, '', name, '', is_admin, created_at FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
