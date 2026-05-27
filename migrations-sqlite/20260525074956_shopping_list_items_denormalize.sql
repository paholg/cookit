-- Reshape shopping_list_items: snapshot the name and grocery_section at
-- generation time so the list survives later ingredient renames and section
-- reassignments. Drop the ingredient_id link entirely and allow ad-hoc items
-- (e.g. "paper towels") with no quantity/unit. The initial schema's version
-- has never been populated, so a destructive rebuild is safe.

DROP TABLE shopping_list_items;

CREATE TABLE shopping_list_items (
    id INTEGER PRIMARY KEY,
    shopping_list_id INTEGER NOT NULL REFERENCES shopping_lists(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    grocery_section TEXT,
    quantity REAL,
    unit_kind TEXT CHECK (unit_kind IS NULL OR unit_kind IN ('mass', 'volume', 'count', 'custom')),
    unit TEXT,
    checked INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL
);
