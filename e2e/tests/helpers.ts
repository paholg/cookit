import { expect, type Page } from "@playwright/test";

// Install a virtual platform authenticator that auto-approves WebAuthn prompts,
// so passkey registration/login resolves without human interaction. Requires a
// Chromium context and a secure-context origin (see the `--unsafely-treat-...`
// launch flag in `playwright.config.ts`).
export async function addVirtualAuthenticator(page: Page): Promise<void> {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("WebAuthn.enable");
  await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
}

// Provision a fresh account, passkey, and cookbook through the real UI, leaving
// the page authenticated on the new book's subdomain. Requires a virtual
// authenticator (see `addVirtualAuthenticator`) and an apex `baseURL`. Returns
// the created user's email and the book slug.
export async function provision(
  page: Page,
): Promise<{ email: string; slug: string }> {
  // Unique even when projects (setup, login) provision concurrently, so the
  // user email and book slug never collide.
  const id = `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
  const email = `e2e-${id}@example.com`;
  const slug = `e2e-${id}`;

  // `networkidle` lets the wasm client load so form handlers are wired up.
  await page.goto("/", { waitUntil: "networkidle" });

  // Create the account.
  await page.getByRole("link", { name: "Create account" }).click();
  const name = page.getByLabel("Name");
  const emailField = page.getByLabel("Email");
  await name.fill(`E2E User ${id}`);
  await emailField.fill(email);
  // Controlled inputs echoing the signal confirms hydration before submit.
  await expect(emailField).toHaveValue(email);
  await page.getByRole("button", { name: "Create account" }).click();

  // Lands on the account page. Register a passkey (the virtual authenticator
  // signs automatically) and wait for the credential to appear before moving on,
  // so login tests have a passkey to authenticate with.
  await page.getByRole("button", { name: "Add passkey" }).click();
  await expect(page.locator("ul.passkey-list li")).toHaveCount(1);

  // New accounts have no cookbook yet; head home to create the first one, which
  // switches to its subdomain and lands on recipes.
  await page.getByRole("link", { name: "CookIt!" }).click();
  await page.getByRole("link", { name: "Create cookbook" }).click();
  const bookName = page.getByLabel("Name");
  const bookUrl = page.getByLabel("Url");
  await bookName.fill(`E2E Cookbook ${id}`);
  await bookUrl.fill(slug);
  await expect(bookUrl).toHaveValue(slug);
  await page.getByRole("button", { name: "Create cookbook" }).click();

  await expect(page.getByRole("heading", { name: "Recipes" })).toBeVisible();

  return { email, slug };
}
