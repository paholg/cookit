FROM mcr.microsoft.com/devcontainers/base:ubuntu

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        build-essential \
        fzf \
        libssl-dev \
        pkg-config \
        python3 \
        sqlite3 \
        zsh-autosuggestions \
        zsh-syntax-highlighting \
    && rm -rf /var/lib/apt/lists/* \
    && chsh -s /usr/bin/zsh vscode

USER vscode
ENV CARGO_HOME=/home/vscode/.cargo \
    RUSTUP_HOME=/home/vscode/.rustup \
    PATH="/home/vscode/.cargo/bin:/home/vscode/.local/bin:${PATH}"

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable \
        --component rust-analyzer --component rust-src \
        --target wasm32-unknown-unknown

# cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash

# tombi
RUN curl -fsSL https://tombi-toml.github.io/tombi/install.sh | sh

RUN cargo binstall --no-confirm \
    atuin \
    cargo-dist \
    cargo-edit \
    cargo-nextest \
    diesel_cli \
    dioxus-cli \
    just \
    && true

COPY --chown=vscode:vscode zshrc /home/vscode/.zshrc

ARG CLAUDE_CODE_CACHE_BUST=replaceme
RUN curl -fsSL https://claude.ai/install.sh | bash \
    && claude --version

WORKDIR /workspace
