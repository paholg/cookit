-- Add migration script here

UPDATE ingredients
SET grocery_section = NULL
WHERE grocery_section IN ('Deli',
                          'Condiments',
                          'Beverages',
                          'Snacks',
                          'Household',
                          'Spices',
                          'Seafood')