# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ENV TZ="America/Los_Angeles" \
    TZDIR="/usr/share/zoneinfo"

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        direnv \
        zsh-autosuggestions \
        zsh-syntax-highlighting \
    && rm -rf /var/lib/apt/lists/* \
    && chsh -s /usr/bin/zsh vscode \
    && ln -snf "/usr/share/zoneinfo/$TZ" /etc/localtime \
    && echo "$TZ" > /etc/timezone \
    && install -d -o vscode -g vscode /nix

USER vscode
ENV CARGO_HOME=/home/vscode/.cargo \
    USER=vscode \
    PATH="/home/vscode/.nix-profile/bin:${PATH}"

# Pre-create the cache directories so the volumes mounted here inherit ownership.
RUN mkdir -p /home/vscode/.cargo/registry \
             /home/vscode/.cargo/git \
             /home/vscode/.cache/sccache

COPY --chown=vscode:vscode zshrc /home/vscode/.zshrc

WORKDIR /workspace
