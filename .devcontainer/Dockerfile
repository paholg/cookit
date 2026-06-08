# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        direnv \
        zsh-autosuggestions \
        zsh-syntax-highlighting \
    && rm -rf /var/lib/apt/lists/* \
    && chsh -s /usr/bin/zsh vscode

USER vscode
ENV CARGO_HOME=/home/vscode/.cargo \
    USER=vscode \
    PATH="/home/vscode/.nix-profile/bin:${PATH}"

# Pre-create the cache directories so the volumes mounted here inherit ownership.
RUN mkdir -p /home/vscode/.cargo/registry \
             /home/vscode/.cargo/git \
             /home/vscode/.cache/sccache

# Install single-user nix
RUN curl --proto '=https' --tlsv1.2 -sSf -L https://nixos.org/nix/install | sh -s -- --no-daemon \
    && mkdir -p /home/vscode/.config/nix \
    && echo 'experimental-features = nix-command flakes' >> /home/vscode/.config/nix/nix.conf

# Install nix-direnv
RUN . /home/vscode/.nix-profile/etc/profile.d/nix.sh \
    && nix profile add nixpkgs#nix-direnv \
    && mkdir -p /home/vscode/.config/direnv \
    && echo 'source $HOME/.nix-profile/share/nix-direnv/direnvrc' > /home/vscode/.config/direnv/direnvrc

COPY --chown=vscode:vscode zshrc /home/vscode/.zshrc

WORKDIR /workspace
