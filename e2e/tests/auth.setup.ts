import { test as setup } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { addVirtualAuthenticator, provision } from "./helpers";

const authDir = path.join(__dirname, "..", ".auth");
const STORAGE_STATE = path.join(authDir, "state.json");
const PROVISIONED = path.join(authDir, "provisioned.json");

// Provision a fresh user + passkey + cookbook through the real UI and persist
// the resulting session so every test in the `chromium` project starts logged
// in on that book. The book slug is recorded for the book-host fixture. This
// replaces the old `/api/dev/*` endpoints — provisioning now goes through the
// same passkey flow a real user would.
setup("provision account", async ({ page }) => {
  await addVirtualAuthenticator(page);

  const { email, slug } = await provision(page);

  mkdirSync(authDir, { recursive: true });
  writeFileSync(PROVISIONED, JSON.stringify({ email, slug }), "utf8");

  await page.context().storageState({ path: STORAGE_STATE });
});
