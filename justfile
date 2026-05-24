set dotenv-load := true

serve *args:
    cd packages/web && DATABASE_URL="sqlite://{{justfile_directory()}}/dev/cookit.db?mode=rwc" dx serve {{args}}

check: lint test

build *args:
    cd packages/web && dx build {{args}}

test *args:
    cargo nextest run --workspace --all-targets --no-fail-fast {{args}}

up:
    nix flake update
    cargo upgrade -i

fix:
    cargo clippy --fix --allow-staged
    cargo fmt --all

lint: fmt-check clippy

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --features web -- -D warnings
    cargo clippy --workspace --all-targets --features server -- -D warnings
