import { test, expect } from "@playwright/test";

// Account provisioning happens on the apex host while unauthenticated, so this
// spec runs in the `provision` project (no storageState, apex baseURL, apex
// origin treated as secure). It drives the full flow:
//   Create account → name/email form → create user (+ login) → create passkey.
// A CDP virtual authenticator stands in for a real platform authenticator so
// `navigator.credentials.create` resolves without human interaction.

test("create an account and register a passkey", async ({ page }) => {
  // Install a virtual platform authenticator that auto-approves prompts.
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

  const name = `E2E Provision ${Date.now()}`;
  const email = `e2e-provision-${Date.now()}@example.com`;

  // `networkidle` lets the wasm client finish loading so the form's event
  // handlers are wired up before we interact with it.
  await page.goto("/", { waitUntil: "networkidle" });
  await page.getByRole("link", { name: "Create account" }).click();

  await expect(
    page.getByRole("heading", { name: "Create account" }),
  ).toBeVisible();

  const nameInput = page.getByLabel("Name");
  const emailInput = page.getByLabel("Email");
  await nameInput.fill(name);
  await emailInput.fill(email);
  // Confirm hydration took (controlled inputs echo the signal) before submit.
  await expect(nameInput).toHaveValue(name);
  await expect(emailInput).toHaveValue(email);

  await page.getByRole("button", { name: "Create account" }).click();

  // Lands on the passkey step.
  await expect(
    page.getByRole("heading", { name: "Create a passkey" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Create passkey" }).click();

  // On success we're redirected home, signed in but without a book yet.
  await expect(page.getByText(/no book yet/i)).toBeVisible();
});
