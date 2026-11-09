#!/usr/bin/env bash
set -euo pipefail

# Setup and restart dnsmasq
sudo cp .devcontainer/dnsmasq.conf /etc/dnsmasq.d/cookit.conf
sudo pkill dnsmasq 2>/dev/null || true
sudo dnsmasq --conf-file=/etc/dnsmasq.d/cookit.conf
echo "nameserver 127.0.0.1" | sudo tee /etc/resolv.conf >/dev/null

# Collect garbage from the shared nix store volume. Every worktree roots its dev
# shell under /nix/var/nix/gcroots/cookit (see .envrc), so this only deletes
# paths no worktree uses.
nix-collect-garbage -d
