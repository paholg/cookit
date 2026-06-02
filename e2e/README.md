# End-to-end tests

Playwright smoke tests that drive the full stack (SSR + hydration + server
functions + Postgres) through a real browser.

## Running

```sh
just test-e2e
```

That runs `npm ci` and `npx playwright test`. Everything else is automatic:

- **Dedicated database.** The Postgres database at `DATABASE_TEST_URL` is created
  and migrated (`global-setup.ts` → `diesel database setup`), then wiped and
  seeded with a single admin user/book/role (`seed e2e-setup`). It is emptied
  again afterwards (`global-teardown.ts` → `seed e2e-teardown`). `DATABASE_TEST_URL`
  is provided by the Nix dev shell (`flake.nix`) and the dev container
  (`docker-compose.yml`) and points at a dedicated test database, so the dev
  database is never touched. It is required — there is no fallback.
- **Login.** The `setup` project (`tests/auth.setup.ts`) logs in as the seeded
  `user_role` via `POST /api/auth/login` and saves the session cookie to
  `.auth/state.json`; every other test reuses it, so they run as the admin.
- **Server.** Playwright starts its own `dx serve` on port **8123** (bound to
  `DATABASE_TEST_URL`), independent of any dev server on 8080. The first build
  compiles the wasm client and can take a few minutes.

## Dependencies

Node, the Chromium build, and `diesel` come from either the dev container
(`.devcontainer/Dockerfile`) or the Nix dev shell (`flake.nix`). In Nix the
browser is supplied via `PLAYWRIGHT_BROWSERS_PATH`; keep the `@playwright/test`
version in `package.json` in sync with both the flake's `playwright-driver` and
the Dockerfile's pinned `playwright@<ver>`.
