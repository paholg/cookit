import { execFileSync } from "node:child_process";
import path from "node:path";

const TEST_DATABASE_URL =
  process.env.TEST_DATABASE_URL ??
  "postgres://postgres:postgres@postgres:5432/cookit_e2e";

const repoRoot = path.join(__dirname, "..");

// Empty the test database so nothing from this run persists.
export default async function globalTeardown() {
  const env = { ...process.env, DATABASE_URL: TEST_DATABASE_URL };

  execFileSync("cargo", ["run", "--quiet", "-p", "seed", "--", "e2e-teardown"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  });
}
