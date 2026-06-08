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
  webServer: {
    // `--features development` exposes the /api/dev/* endpoints the setup and
    // cleanup projects use to create and delete their test data. Migrations run
    // automatically on server start, so there's no diesel/cargo step.
    //
    // Disable live reloading: e2e runs a fixed build, not an interactive dev
    // session. (`--hot-patch` is a bare flag that already defaults to off and
    // isn't affected by dx CLI settings, so we just don't pass it.)
    command: `dx serve -p web --features development --addr 127.0.0.1 --port ${PORT} --hot-reload false`,
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
