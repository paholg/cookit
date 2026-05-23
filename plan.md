CookIt will be a self-hosted recipe service.

It is a dioxus app with a sqlite database using sqlx.

For the final product:
* A user will be able to quickly select several recipes to create a meal, with
 potential multipliers (e.g. 1x chili, 2x cornbread, 0.5x winter salad), which
 will render a page where it's easy to switch tabs for these recipes to cook.
* We can generate a shopping list from a meal, which will be able to combine
 ingredients (if recipe 1 takes an onion, and recipe 2 takes 0.5 onions, the
 shopping list will show 1.5 onions). The shopping list will be interactive,
 letting the user check things as they get them.
* Recipes will store their ingredients per step.
* Ingredients will have an optional density, to let users show recipes in
 volume or mass regardless of what the source has. We will always store recipes
 with the source units. Ingredients will also have grocery store sections, so
 they can be grouped for shopping lists.
* We won't worry about auth yet, but we'll do this with a 3rd party via OIDC or
 something. For now, we should just have a users table, and meals and shopping
 lists should be attached to users.
* accessing meals and shopping_lists should be a very nice experience on a phone
  or tablet.

We want to store data in a highly relational way for easier querying.

Some base tables:

* ingredients have a name, density (optional), grocery store location (optional)
* recipes has a name, source (url or text description), and many steps
* recipe_steps have instruction text, position, and have many ingredients with quantities
  (in mass, volume, count, or custom. We should be able to support "1 medium onion" or "2 tsp kosher salt" vs "2 tsp table salt".

* meals have many recipes with a multiplier and position
* shopping_lists have many ingredients with quantities

Some of these will require join tables.
