serve *args:
    cd packages/web && DATABASE_URL="sqlite://{{justfile_directory()}}/dev/cookit.db?mode=rwc" dx serve {{args}}

build *args:
    cd packages/web && dx build {{args}}

test *args:
    cargo nextest run --no-fail-fast --features server {{args}}

up:
    nix flake update
    cargo upgrade -i

fix:
    cargo clippy --fix --allow-staged
    cargo fmt

lint: fmt-check clippy

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --features web -- -D warnings
    cargo clippy --workspace --features server -- -D warnings
