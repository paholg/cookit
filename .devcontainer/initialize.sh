#!/usr/bin/env bash
set -euo pipefail

truncate -s0 .devcontainer/.env

# Ensure external volumes exist
for vol in cookit-nix-store cookit-cargo-registry cookit-cargo-git cookit-sccache; do
    docker volume create "$vol" 2>/dev/null || true
done

echo "WORKSPACE_DIR=$(basename "$(pwd)")" >> .devcontainer/.env
