import { execFileSync } from "node:child_process";
import path from "node:path";
import { databaseUrl } from "./db-url";

const repoRoot = path.join(__dirname, "..");

// Empty the test database so nothing from this run persists.
export default async function globalTeardown() {
  const env = { ...process.env, DATABASE_URL: databaseUrl() };

  execFileSync("cargo", ["run", "--quiet", "-p", "seed", "--", "e2e-teardown"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  });
}
