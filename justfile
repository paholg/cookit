set dotenv-load := true

serve *args:
    dx serve -p web --port 8080 {{args}}

check: lint test

# Browser end-to-end tests.
e2e *args:
    cd e2e && npm ci && npx playwright test {{args}}

seed:
    cargo run --bin seed

download-colors:
    ./script/download-colors mauve cyan mint amber ruby

db-setup:
    diesel setup

db-migrate:
    diesel migration run

db-redo:
    diesel migration redo

db-revert:
    diesel migration revert

build *args:
    cd crates/web && dx build {{args}}

test *args:
    cargo nextest run --workspace --all-targets --no-fail-fast {{args}}


up:
    nix flake update
    # `dx` and the `dioxus` crate must be the same version, and nixpkgs can lag
    # crates.io. Update nixpkgs first, then cap `dioxus` at whatever `dx` it
    # gives us; everything else goes to the latest it can.
    cargo upgrade -i --exclude dioxus
    cargo upgrade -p dioxus@$(nix eval --raw .#dioxusCli.version)
    cd e2e && npm update

fmt:
    # Can break rsx :(
    # dx fmt
    cargo fmt --all
    tombi format

fix:
    cargo clippy --workspace --all-targets --all-features --allow-staged --fix
    just fmt
    script/schema-dump
    cargo machete --fix

lint: fmt-check clippy
    cargo machete

fmt-check:
    cargo fmt --all -- --check
    tombi format --check

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy --workspace --all-targets --features web -- -D warnings
    cargo clippy --workspace --all-targets --features server -- -D warnings
