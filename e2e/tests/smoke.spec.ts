import { test, expect } from "@playwright/test";

// These run against a live `dx serve` (started automatically by the Playwright
// config) backed by the dev Postgres database. Each test that creates data
// deletes it again so the suite is safe to re-run.

test("home renders the recipe list", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();
});

test("primary navigation reaches every section", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("link", { name: "Meals", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Meals" })).toBeVisible();

  await page.getByRole("link", { name: "Shopping", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Shopping lists" }),
  ).toBeVisible();

  await page.getByRole("link", { name: "Ingredients", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Ingredients" })).toBeVisible();
});

test("create then delete a recipe", async ({ page }) => {
  const name = `E2E Recipe ${Date.now()}`;

  await page.goto("/recipes/new");
  await expect(page.getByRole("heading", { name: "New recipe" })).toBeVisible();

  await page.locator('[data-focus-key="recipe-name"]').fill(name);
  await page.locator("textarea").first().fill("Mix everything together.");
  await page.getByRole("button", { name: "Save recipe" }).click();

  // Lands on the recipe's detail page, titled with its name.
  await expect(page.getByRole("heading", { name })).toBeVisible();

  // Clean up: edit -> delete (confirm dialog), back to the list.
  await page.getByRole("button", { name: "Edit recipe" }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Delete recipe" }).click();
  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();
});

test("create then delete an empty shopping list", async ({ page }) => {
  const name = `E2E List ${Date.now()}`;

  await page.goto("/shopping-lists/new");
  await page.getByPlaceholder("List name").fill(name);
  await page.getByRole("button", { name: "Create" }).click();

  await expect(page.getByRole("heading", { name })).toBeVisible();

  // Clean up from the list page.
  await page.getByRole("link", { name: "Shopping", exact: true }).click();
  const row = page.locator("li", { hasText: name });
  page.once("dialog", (dialog) => dialog.accept());
  await row.getByRole("button", { name: "Delete shopping list" }).click();
  await expect(row).toHaveCount(0);
});
