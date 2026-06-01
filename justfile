set dotenv-load := true

serve *args:
    dx serve -p web {{args}}

db-setup:
    diesel setup

db-migrate:
    diesel migration run

db-redo:
    diesel migration redo

db-revert:
    diesel migration revert

check: lint test

build *args:
    cd crates/web && dx build {{args}}

test *args:
    cargo nextest run --workspace --all-targets --no-fail-fast {{args}}

up:
    nix flake update
    cargo upgrade -i

fix:
    cargo clippy --fix --allow-staged
    # Can break rsx :(
    # dx fmt
    cargo fmt --all
    tombi format

lint: fmt-check clippy

fmt-check:
    cargo fmt --all -- --check
    tombi format --check

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy --workspace --all-targets --features web -- -D warnings
    cargo clippy --workspace --all-targets --features server -- -D warnings
