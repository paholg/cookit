-- Optional countdown timer per step. Null = no timer offered for that step.
ALTER TABLE recipe_steps ADD COLUMN duration_seconds INTEGER;
