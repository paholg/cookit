import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
import { freePort } from "./free-port";

// Caddy listens on PORT and reverse-proxies to `dx serve` on INTERNAL_PORT,
// adding X-Forwarded-Host for the server.
const PORT = Number.parseInt(
  (process.env.E2E_PORT ??= String(freePort())),
  10,
);
const INTERNAL_PORT = freePort();

const BASE_URL = `http://127.0.0.1:${PORT}`;
const INTERNAL_URL = `http://127.0.0.1:${INTERNAL_PORT}`;

// The webauthn domain
const APEX_URL = `http://${process.env.BASE_DOMAIN!}:${PORT}`;

// Passkey flows run on the apex host over plain HTTP, so treat that origin as
// secure for `navigator.credentials`. The tests add a virtual authenticator.
const APEX_USE = {
  ...devices["Desktop Chrome"],
  baseURL: APEX_URL,
  launchOptions: {
    args: [`--unsafely-treat-insecure-origin-as-secure=${APEX_URL}`],
  },
};

const STORAGE_STATE = path.join(__dirname, ".auth", "state.json");

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
    // Full Chromium, not `chrome-headless-shell`: passkey registration never
    // resolves in the shell, so the naming dialog never opens. Still true on
    // browser rev 1228; the older flex-rendering bug is fixed.
    channel: "chromium",
  },
  projects: [
    // Provisions a fresh user + passkey + cookbook through the real UI and saves
    // the session for the chromium project to reuse.
    {
      name: "setup",
      testMatch: /auth\.setup\.ts/,
      use: APEX_USE,
    },
    // Passkey login round-trip; provisions its own account to stay independent
    // of the shared setup session.
    {
      name: "login",
      testMatch: /login\.spec\.ts/,
      use: APEX_USE,
    },
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], storageState: STORAGE_STATE },
      dependencies: ["setup"],
      testIgnore: /(auth\.setup|login\.spec)\.ts/,
    },
  ],
  webServer: [
    {
      command: `dx serve -p web --addr 127.0.0.1 --port ${INTERNAL_PORT} --hot-reload false --watch false`,
      cwd: "..",
      url: INTERNAL_URL,
      // The server sits behind Caddy and can't infer its public origin, so tell
      // it the one the browser uses or webauthn rejects the credential.
      env: { WEBAUTHN__ORIGIN: APEX_URL },
      reuseExistingServer: false,
      timeout: 600_000,
      stdout: "pipe",
      stderr: "pipe",
    },
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
