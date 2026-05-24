set dotenv-load := true

serve *args:
    cd packages/web && DATABASE_URL="sqlite://{{justfile_directory()}}/dev/cookit.db?mode=rwc" dx serve --features dev-auth {{args}}

prepare:
    cargo sqlx prepare --workspace -- --all-targets --all-features

check: lint test check-sqlx

build *args:
    cd packages/web && dx build {{args}}

test *args:
    cargo nextest run --workspace --all-targets --no-fail-fast {{args}}

up:
    nix flake update
    cargo upgrade -i

fix:
    just prepare
    cargo clippy --fix --allow-staged
    cargo fmt --all
    tombi format

lint: fmt-check clippy

check-sqlx:
    cargo sqlx prepare --check --workspace -- --all-targets --all-features

fmt-check:
    cargo fmt --all -- --check
    tombi format --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --features web -- -D warnings
    cargo clippy --workspace --all-targets --features server -- -D warnings
    cargo clippy --workspace --all-targets --features "server dev-auth" -- -D warnings
