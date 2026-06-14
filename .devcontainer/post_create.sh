#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------------------------
# Setup nix

# Store nix profile in /nix, which comes from a docker volume, so is persisted.
NIX_STATE=/nix/var/nix-user-state
mkdir -p "$NIX_STATE" ~/.local/state ~/.config/nix ~/.config/direnv
ln -sfn "$NIX_STATE" ~/.local/state/nix

echo 'experimental-features = nix-command flakes' > ~/.config/nix/nix.conf

if [[ ! -e ~/.local/state/nix/profiles/profile/bin/nix ]]; then
    echo "Cold /nix volume; installing single-user nix..."
    curl --proto '=https' --tlsv1.2 -sSf -L https://nixos.org/nix/install \
        | sh -s -- --no-daemon --no-modify-profile
fi

# Relink ~/.nix-profile into the volume.
ln -sfn ~/.local/state/nix/profiles/profile ~/.nix-profile
# shellcheck disable=SC2016
echo 'source $HOME/.nix-profile/share/nix-direnv/direnvrc' > ~/.config/direnv/direnvrc

# shellcheck source=/dev/null
. ~/.nix-profile/etc/profile.d/nix.sh

if [[ ! -e ~/.nix-profile/share/nix-direnv/direnvrc ]]; then
    nix profile add nixpkgs#nix-direnv
fi

direnv allow
# shellcheck source=/dev/null
source <(direnv export zsh)

# ------------------------------------------------------------------------------
# Now that nix is setup, run actual post-create commands:
just db-setup
