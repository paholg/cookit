import { test, expect } from "@playwright/test";
import { addVirtualAuthenticator, provision } from "./helpers";

// Exercises the username-first passkey login. Runs in the `login` project (apex
// host, secure origin, no saved session) and provisions its own account first so
// there's a passkey to authenticate with — all against the same virtual
// authenticator, so the registered credential is available at login.
test("log in with a passkey", async ({ page }) => {
  await addVirtualAuthenticator(page);

  const { email } = await provision(page);

  // Log out from the account menu, which reloads the apex landing. Let the
  // client hydrate first so the menu trigger's handler is wired up (the cookbook
  // step above was a full page load onto the book's subdomain).
  await page.waitForLoadState("networkidle");
  await page.getByRole("button", { name: "Account menu" }).click();
  await page.getByRole("option", { name: "Log out" }).click();

  // Go to the login page and let it hydrate before interacting — logout was a
  // full page load, so the form's submit handler isn't wired up until the wasm
  // client reloads (otherwise the click falls through to a native form GET).
  await page.getByRole("link", { name: "Log in" }).click();
  await page.waitForLoadState("networkidle");

  // Log back in via the passkey flow.
  await page.getByLabel("Email").fill(email);
  await page.getByRole("button", { name: "Log in" }).click();

  // Authenticated again and dropped back into the book.
  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();
});
