import { test as base, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { bookBaseURL } from "./book-host";

const DEV_DATA = path.join(__dirname, "..", ".auth", "dev.json");

// The test book's slug is random per run (created by `auth.setup.ts`), so the
// book host can't be a static config value. Override `baseURL` from the slug
// recorded in `dev.json` so every `page.goto("/…")` lands on the right book.
export const test = base.extend({
  baseURL: async ({}, use) => {
    const { slug } = JSON.parse(readFileSync(DEV_DATA, "utf8"));
    await use(bookBaseURL(slug));
  },
});

export { expect };
