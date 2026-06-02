truncate -s0 .devcontainer/.env

# Ensure we have a fresh claude code version.
CLAUDE_CODE_CACHE_BUST=$(curl -sf https://api.github.com/repos/anthropics/claude-code/releases/latest | jq -r .tag_name)
echo "CLAUDE_CODE_CACHE_BUST=\"$CLAUDE_CODE_CACHE_BUST\"" >> .devcontainer/.env

echo "WORKSPACE_DIR=$(basename "$(pwd)")" >> .devcontainer/.env

# Pin dioxus-cli in the image to the dioxus version resolved in Cargo.lock, so
# the `dx` CLI stays in lockstep with the library.
DIOXUS_VERSION=$(grep -A1 '^name = "dioxus"$' Cargo.lock | grep '^version' | head -1 | cut -d'"' -f2)
echo "DIOXUS_VERSION=\"$DIOXUS_VERSION\"" >> .devcontainer/.env

# If gh is available, capture a token so `cargo binstall` requests are
# authenticated.
if command -v gh >/dev/null 2>&1; then
    GITHUB_TOKEN_FILE="${TMPDIR:-/tmp}/cookit-github-token-$(basename "$(pwd)")"
    : > "$GITHUB_TOKEN_FILE"
    chmod 600 "$GITHUB_TOKEN_FILE"
    gh auth token >> "$GITHUB_TOKEN_FILE" 2>/dev/null || true
    echo "GITHUB_TOKEN_FILE=\"$GITHUB_TOKEN_FILE\"" >> .devcontainer/.env
fi

