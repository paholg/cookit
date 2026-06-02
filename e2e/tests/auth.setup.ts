import { test as setup, expect } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const authDir = path.join(__dirname, "..", ".auth");
const STORAGE_STATE = path.join(authDir, "state.json");
const DEV_DATA = path.join(authDir, "dev.json");

// Create an isolated admin user/book/role via the development endpoint, then log
// in as it and persist the session cookie so every test in the `chromium`
// project starts authenticated. The created ids are written to `dev.json` for
// the `cleanup` teardown project to delete afterwards.
setup("authenticate", async ({ page }) => {
  const setupRes = await page.request.post("/api/dev/setup");
  expect(setupRes.ok()).toBeTruthy();

  const data = await setupRes.json();

  mkdirSync(authDir, { recursive: true });
  writeFileSync(DEV_DATA, JSON.stringify(data), "utf8");

  const loginRes = await page.request.post("/api/auth/login", {
    data: { user_role_id: data.user_role_id },
  });
  expect(loginRes.ok()).toBeTruthy();

  await page.context().storageState({ path: STORAGE_STATE });
});
