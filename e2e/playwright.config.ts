import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
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

// `dx serve` runs on an internal port; Caddy sits in front on PORT and adds the
// `X-Forwarded-Host` header (from the request's Host) that the server resolves
// the book from — standing in for the production/devcontainer proxy.
const INTERNAL_PORT = freePort();
const INTERNAL_URL = `http://127.0.0.1:${INTERNAL_PORT}`;

const STORAGE_STATE = path.join(__dirname, ".auth", "state.json");

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
    // Use the full Chromium (new headless mode), not `chrome-headless-shell`.
    // The Nix-pinned shell (revision 1217) mis-renders flex items.
    // Should be fixed by version 1223.
    channel: "chromium",
  },
  projects: [
    // Creates an isolated test user/book/role via /api/dev/setup, logs in, and
    // saves the session cookie. Its `teardown` deletes that data once every
    // dependent project has finished (the web server is still up at that point).
    {
      name: "setup",
      testMatch: /auth\.setup\.ts/,
      teardown: "cleanup",
    },
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], storageState: STORAGE_STATE },
      dependencies: ["setup"],
      testIgnore: /(auth\.setup|cleanup\.teardown)\.ts/,
    },
    // Deletes the test user/book via /api/dev/teardown.
    { name: "cleanup", testMatch: /cleanup\.teardown\.ts/ },
  ],
  webServer: [
    {
      // Disable live reloading and the filesystem watcher, so the server stays
      // static during the test run.
      command: `dx serve -p web --features development --addr 127.0.0.1 --port ${INTERNAL_PORT} --hot-reload false`,
      cwd: "..",
      url: INTERNAL_URL,
      // Each run starts a server with an OS-provided port.
      reuseExistingServer: false,
      timeout: 600_000,
      stdout: "pipe",
      stderr: "pipe",
    },
    // Run Caddy in front of dioxus, so that X-Forwarded-Host gets set.
    {
      command: `caddy reverse-proxy --from :${PORT} --to 127.0.0.1:${INTERNAL_PORT}`,
      url: BASE_URL,
      reuseExistingServer: false,
      timeout: 60_000,
      stdout: "pipe",
      stderr: "pipe",
    },
  ],
});
