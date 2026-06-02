import { test as setup, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";

const authDir = path.join(__dirname, "..", ".auth");
const STORAGE_STATE = path.join(authDir, "state.json");

// Log in as the user_role seeded by global-setup, then persist the resulting
// session cookie so every test in the `chromium` project starts authenticated.
setup("authenticate", async ({ page }) => {
  const roleId = readFileSync(path.join(authDir, "role-id"), "utf8").trim();

  const res = await page.request.post("/api/auth/login", {
    data: { user_role_id: roleId },
  });
  expect(res.ok()).toBeTruthy();

  await page.context().storageState({ path: STORAGE_STATE });
});
