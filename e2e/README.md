# End-to-end tests

Playwright smoke tests that drive the full stack (SSR + hydration + server
functions + Postgres) through a real browser.

## Running

```sh
just test-e2e
```

That runs `npm ci` and `npx playwright test`. Everything else is automatic:

- **Test data via endpoints.** No `diesel`/`cargo` and no separate database. The
  `setup` project (`tests/auth.setup.ts`) calls `POST /api/dev/setup` to create
  an isolated admin user/book/role with a unique-per-run email/slug, then logs
  in via `POST /api/auth/login` and saves the session cookie to
  `.auth/state.json`. The `cleanup` teardown project (`tests/cleanup.teardown.ts`)
  calls `POST /api/dev/teardown` afterwards; deleting the book cascades the role
  and every book-scoped row, so the dev database is left as it was. These
  endpoints only exist when the server is built with `--features development`,
  which the Playwright config passes to its `dx serve`.
- **Login reuse.** Every test in the `chromium` project reuses the saved cookie,
  so they run as the created admin against a fresh, empty book.
- **Server.** Playwright starts its own `dx serve` on an OS-assigned free port,
  using the ambient `DATABASE_URL`. Migrations run automatically on server
  start. The first build compiles the wasm client and can take a few minutes.
