# End-to-end tests

Playwright smoke tests that drive the full stack (SSR + hydration + server
functions + Postgres) through a real browser.

## Running

```sh
just e2e
```

That runs `npm ci` and `npx playwright test`. Everything else is automatic:

- **Provisioning through the real UI.** No `diesel`/`cargo`, no separate
  database, and no special endpoints. The `setup` project (`tests/auth.setup.ts`,
  via the `provision` helper in `tests/helpers.ts`) drives the actual
  create-account → register-passkey → create-cookbook flow with a unique-per-run
  email/slug, using a CDP virtual authenticator for the passkey. It saves the
  session to `.auth/state.json` and the book slug to `.auth/provisioned.json`.
  Because passkeys need a secure context, the `setup` project treats the apex
  origin as secure and the server is told its real origin via `WEBAUTHN__ORIGIN`.
- **Login reuse.** Every test in the `chromium` project reuses the saved cookie,
  so they run as the provisioned admin against a fresh, empty book.
- **No teardown.** Each run leaves its user/passkey/book behind (unique per run);
  there's no dev endpoint to delete it.
- **Server.** Playwright starts its own `dx serve` on an OS-assigned free port,
  using the ambient `DATABASE_URL`. Migrations run automatically on server
  start. The first build compiles the wasm client and can take a few minutes.
