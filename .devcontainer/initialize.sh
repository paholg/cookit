truncate -s0 .devcontainer/.env

# Ensure we have a fresh claude code version.
CLAUDE_CODE_CACHE_BUST=$(curl -sf https://api.github.com/repos/anthropics/claude-code/releases/latest | jq -r .tag_name)
echo "CLAUDE_CODE_CACHE_BUST=\"$CLAUDE_CODE_CACHE_BUST\"" >> .devcontainer/.env

echo "WORKSPACE_DIR=$(basename "$(pwd)")" >> .devcontainer/.env

