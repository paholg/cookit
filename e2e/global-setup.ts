import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const TEST_DATABASE_URL =
  process.env.TEST_DATABASE_URL ??
  "postgres://postgres:postgres@postgres:5432/cookit_e2e";

const repoRoot = path.join(__dirname, "..");
const authDir = path.join(__dirname, ".auth");

// Create + migrate the dedicated test database, then wipe-and-seed a single
// admin user/book/role. The seed prints `USER_ROLE_ID=<id>`, which we hand to
// `auth.setup.ts` to log in with.
export default async function globalSetup() {
  const env = { ...process.env, DATABASE_URL: TEST_DATABASE_URL };

  // Idempotent: creates the DB if missing and runs migrations. `--locked-schema`
  // guards against accidentally regenerating the committed schema.rs.
  execFileSync("diesel", ["database", "setup", "--locked-schema"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  });

  const out = execFileSync(
    "cargo",
    ["run", "--quiet", "-p", "seed", "--", "e2e-setup"],
    { cwd: repoRoot, env, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );

  const match = out.match(/USER_ROLE_ID=(\S+)/);
  if (!match) {
    throw new Error(`seed e2e-setup did not print USER_ROLE_ID:\n${out}`);
  }

  mkdirSync(authDir, { recursive: true });
  writeFileSync(path.join(authDir, "role-id"), match[1], "utf8");
}
