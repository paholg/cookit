// The dedicated test database. Provided by the Nix dev shell (flake.nix) and the
// dev container (docker-compose.yml) as DATABASE_TEST_URL.
//
// No fallback on purpose: a missing value means the shell/container wasn't set
// up, and we want that surfaced loudly rather than silently pointing the suite
// (which creates, seeds, and wipes its database) at some default.
export function databaseUrl(): string {
  const url = process.env.DATABASE_TEST_URL;

  if (!url) {
    throw new Error(
      "DATABASE_TEST_URL is not set. It is provided by the Nix dev shell " +
        "(flake.nix) and the dev container (docker-compose.yml); enter one of " +
        "those before running the e2e suite.",
    );
  }

  return url;
}
