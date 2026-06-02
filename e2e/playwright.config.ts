import { defineConfig, devices } from "@playwright/test";
import path from "node:path";

// A dedicated port so the e2e server never collides with (or gets reused in
// place of) a dev `dx serve` on 8080 that points at the dev database.
const PORT = 8123;
const BASE_URL = `http://127.0.0.1:${PORT}`;

// Dedicated test database — the suite creates, seeds, and wipes it, so it never
// touches dev data. `global-setup.ts` and the seed binary read the same var.
const TEST_DATABASE_URL =
  process.env.TEST_DATABASE_URL ??
  "postgres://postgres:postgres@postgres:5432/cookit_e2e";

const STORAGE_STATE = path.join(__dirname, ".auth", "state.json");

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
  },
  // Create + migrate + seed the test DB before everything; wipe it after.
  globalSetup: "./global-setup.ts",
  globalTeardown: "./global-teardown.ts",
  projects: [
    // Logs in as the seeded user_role and saves the session cookie.
    { name: "setup", testMatch: /auth\.setup\.ts/ },
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], storageState: STORAGE_STATE },
      dependencies: ["setup"],
      testIgnore: /auth\.setup\.ts/,
    },
  ],
  webServer: {
    command: `dx serve -p web --addr 127.0.0.1 --port ${PORT}`,
    cwd: "..",
    url: BASE_URL,
    // Safe to reuse: this port only ever hosts an e2e server bound to the test
    // DB. The first build compiles the wasm client and can take minutes.
    reuseExistingServer: true,
    timeout: 600_000,
    env: {
      DATABASE_URL: TEST_DATABASE_URL,
    },
  },
});
