# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

# Node.js, for the Playwright end-to-end tests.
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash -

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        build-essential \
        fzf \
        libpq-dev \
        libssl-dev \
        nodejs \
        pkg-config \
        python3 \
        sqlite3 \
        zsh-autosuggestions \
        zsh-syntax-highlighting \
    && rm -rf /var/lib/apt/lists/* \
    && chsh -s /usr/bin/zsh vscode

# Install playwright browsers using version from package-lock.json.
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
ARG PLAYWRIGHT_VERSION
RUN npx --yes playwright@${PLAYWRIGHT_VERSION} install --with-deps chromium \
    && chmod -R a+rX /ms-playwright

USER vscode
ENV CARGO_HOME=/home/vscode/.cargo \
    RUSTUP_HOME=/home/vscode/.rustup \
    PATH="/home/vscode/.cargo/bin:/home/vscode/.local/bin:${PATH}"

# Pre-create the cache directories so the volumes mounted here inherit ownership.
RUN mkdir -p /home/vscode/.cargo/registry \
             /home/vscode/.cargo/git \
             /home/vscode/.cache/sccache

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable \
        --component rust-analyzer --component rust-src \
        --target wasm32-unknown-unknown


# tombi
RUN curl -fsSL https://tombi-toml.github.io/tombi/install.sh | sh

# Set token for cargo-binstall to prevent rate-limiting.
RUN --mount=type=secret,id=github_token,uid=1000 \
    GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash

RUN --mount=type=secret,id=github_token,uid=1000 \
    GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    cargo binstall --no-confirm \
    atuin \
    cargo-edit \
    cargo-nextest \
    diesel_cli \
    just \
    sccache \
    && true

ARG DIOXUS_VERSION
RUN --mount=type=secret,id=github_token,uid=1000 \
    GITHUB_TOKEN="$(cat /run/secrets/github_token 2>/dev/null || true)" \
    cargo binstall --no-confirm dioxus-cli@${DIOXUS_VERSION}

COPY --chown=vscode:vscode zshrc /home/vscode/.zshrc

ARG CLAUDE_CODE_CACHE_BUST=replaceme
RUN curl -fsSL https://claude.ai/install.sh | bash \
    && claude --version

WORKDIR /workspace
