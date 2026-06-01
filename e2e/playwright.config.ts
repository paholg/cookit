import { defineConfig, devices } from "@playwright/test";

const PORT = 8080;
const BASE_URL = `http://127.0.0.1:${PORT}`;

// Start `dx serve` automatically unless a server is already listening. The first
// build compiles the wasm client and can take several minutes, hence the long
// timeout.
export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
  ],
  webServer: {
    command: `dx serve -p web --addr 127.0.0.1 --port ${PORT}`,
    cwd: "..",
    url: BASE_URL,
    reuseExistingServer: true,
    timeout: 600_000,
    env: {
      DATABASE_URL:
        process.env.DATABASE_URL ??
        "postgres://postgres:postgres@postgres:5432/cookit",
    },
  },
});
