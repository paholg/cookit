-- Split recipe_steps.instruction into its own table so a step can carry
-- multiple ordered instructions (mirroring recipe_step_ingredients).
CREATE TABLE recipe_step_instructions (
    id INTEGER PRIMARY KEY,
    step_id INTEGER NOT NULL REFERENCES recipe_steps(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    text TEXT NOT NULL,
    UNIQUE (step_id, position)
);

-- Each existing step's single instruction becomes the first row.
INSERT INTO recipe_step_instructions (step_id, position, text)
SELECT id, 0, instruction FROM recipe_steps;

ALTER TABLE recipe_steps DROP COLUMN instruction;
