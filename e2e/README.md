# End-to-end tests

Playwright smoke tests that drive the full stack (SSR + hydration + server
functions + Postgres) through a real browser.

The dev container image already includes Node.js and the Chromium build
(installed by `.devcontainer/Dockerfile` into `/ms-playwright`). The first time
you run the suite, install the JS dependencies:

```sh
cd e2e
npm install
```

Then run it:

```sh
npm test
```

The Playwright config starts `dx serve` automatically (reusing one that's
already running) and points the browser at <http://127.0.0.1:8080>. The first
build compiles the wasm client and can take a few minutes.

`DATABASE_URL` defaults to the dev container's Postgres
(`postgres://postgres:postgres@postgres:5432/cookit`); override it in the
environment to point elsewhere.

Keep the `@playwright/test` version in `package.json` in sync with the version
pinned in `.devcontainer/Dockerfile`.
