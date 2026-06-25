import { test as base, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { bookBaseURL } from "./book-host";

const PROVISIONED = path.join(__dirname, "..", ".auth", "provisioned.json");

// The test book's slug is random per run (created by `auth.setup.ts`), so the
// book host can't be a static config value. Override `baseURL` from the slug
// recorded in `provisioned.json` so every `page.goto("/…")` lands on the right
// book.
export const test = base.extend({
  baseURL: async ({}, use) => {
    const { slug } = JSON.parse(readFileSync(PROVISIONED, "utf8"));
    await use(bookBaseURL(slug));
  },
});

export { expect };
