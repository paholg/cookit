import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
import { databaseUrl } from "./db-url";
import { freePort } from "./free-port";

const PORT = (() => {
  const existing = process.env.E2E_PORT;
  if (existing) {
    return Number.parseInt(existing, 10);
  }

  const port = freePort();
  process.env.E2E_PORT = String(port);
  return port;
})();
const BASE_URL = `http://127.0.0.1:${PORT}`;

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
    // Disable live reloading: e2e runs a fixed build, not an interactive dev
    // session. (`--hot-patch` is a bare flag that already defaults to off and
    // isn't affected by dx CLI settings, so we just don't pass it.)
    command: `dx serve -p web --addr 127.0.0.1 --port ${PORT} --hot-reload false`,
    cwd: "..",
    url: BASE_URL,
    // Each run binds a fresh OS-assigned port, so there's never an existing
    // server to reuse — always start our own. The first build compiles the wasm
    // client and can take minutes.
    reuseExistingServer: false,
    timeout: 600_000,
    // Stream the build/serve output so a slow first compile looks like progress
    // rather than a silent hang.
    stdout: "pipe",
    stderr: "pipe",
  },
});
