import { test, expect } from "./fixtures";

// These run against a live `dx serve` (started automatically by the Playwright
// config, with `--features development`). The `setup` project creates an
// isolated user/book via `/api/dev/setup`, logs in, and saves the session
// cookie, so every test here runs as that admin against a fresh, empty book.
// Tests that create data delete it again so they're re-runnable.

test("home renders the recipe list", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();
});

test("starts authenticated as the seeded admin", async ({ page }) => {
  await page.goto("/");
  // The "Log out" control is only present when a session is active.
  await expect(page.getByRole("button", { name: "Log out" })).toBeVisible();
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

  // `networkidle` lets the wasm client finish downloading; the textarea below
  // lives in a `ClientOnly` wrapper that only renders once hydration has run,
  // so waiting for it confirms the form's event handlers are wired up before
  // we submit (otherwise the click falls through to a native form GET).
  await page.goto("/recipes/new", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "New recipe" })).toBeVisible();

  await page.locator('[data-focus-key="recipe-name"]').fill(name);
  await page.locator("textarea").first().fill("Mix everything together.");
  await page.getByRole("button", { name: "Save recipe" }).click();

  // Lands on the recipe's detail page, titled with its name.
  await expect(page.getByRole("heading", { name })).toBeVisible();

  // Clean up: edit -> delete -> confirm in the in-app AlertDialog, back to list.
  await page.getByRole("button", { name: "Edit recipe" }).click();
  await page.getByRole("button", { name: "Delete recipe" }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();
});

test("create then delete an empty shopping list", async ({ page }) => {
  const name = `E2E List ${Date.now()}`;

  await page.goto("/shopping-lists/new", { waitUntil: "networkidle" });
  const input = page.getByPlaceholder("List name");
  await input.fill(name);
  // Confirm hydration took (controlled input echoes the signal) before submit.
  await expect(input).toHaveValue(name);
  await page.getByRole("button", { name: "Create" }).click();

  await expect(page.getByRole("heading", { name })).toBeVisible();

  // Clean up from the list page; confirm in the in-app AlertDialog.
  await page.getByRole("link", { name: "Shopping", exact: true }).click();
  const row = page.locator("li", { hasText: name });
  await row.getByRole("button", { name: "Delete shopping list" }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(row).toHaveCount(0);
});
