import { test as teardown, expect } from "@playwright/test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";

const authDir = path.join(__dirname, "..", ".auth");
const DEV_DATA = path.join(authDir, "dev.json");

// Delete the user/book created in `auth.setup.ts`. Runs as the `setup` project's
// teardown, so the web server is still up. Cascades remove the role and every
// book-scoped row, leaving the dev database as it was before the run.
teardown("delete test data", async ({ page }) => {
  if (!existsSync(DEV_DATA)) {
    return;
  }

  const { user_id, book_id } = JSON.parse(readFileSync(DEV_DATA, "utf8"));

  const res = await page.request.post("/api/dev/teardown", {
    data: { user_id, book_id },
  });
  expect(res.ok()).toBeTruthy();

  rmSync(DEV_DATA);
});
