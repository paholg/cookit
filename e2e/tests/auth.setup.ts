import { test as setup, expect } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { bookBaseURL } from "./book-host";

const authDir = path.join(__dirname, "..", ".auth");
const STORAGE_STATE = path.join(authDir, "state.json");
const DEV_DATA = path.join(authDir, "dev.json");

// Create an isolated admin user/book/role via the development endpoint, then log
// in as it and persist the session cookie so every test in the `chromium`
// project starts authenticated. The created ids (incl. the book slug) are
// written to `dev.json` for the `cleanup` teardown and the book-host fixture.
//
// TODO: Passkeys
setup("authenticate", async ({ page }) => {
  // `/api/dev/setup` is book-agnostic, so the config's 127.0.0.1 baseURL is fine.
  const setupRes = await page.request.post("/api/dev/setup");
  expect(setupRes.ok()).toBeTruthy();

  const data = await setupRes.json();

  mkdirSync(authDir, { recursive: true });
  writeFileSync(DEV_DATA, JSON.stringify(data), "utf8");

  // Log in *on the book's host* so the session cookie's domain matches and is
  // sent to every book subdomain the tests then visit.
  const loginRes = await page.request.post(
    `${bookBaseURL(data.slug)}/api/auth/login`,
    { data: { user_id: data.user_id } },
  );
  expect(loginRes.ok()).toBeTruthy();

  await page.context().storageState({ path: STORAGE_STATE });
});
