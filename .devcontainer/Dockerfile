FROM mcr.microsoft.com/devcontainers/base:ubuntu

# dpkg fsync()s after unpacking every file, which leaves apt I/O-bound (0% CPU)
# for ages on overlay/devcontainer filesystems. Skip the per-file fsync.
RUN echo 'force-unsafe-io' > /etc/dpkg/dpkg.cfg.d/02apt-speedup

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

# Pre-install the Playwright Chromium build and its OS dependencies into a
# world-readable location so the vscode user can run the e2e suite without a
# separate browser download. Keep this version in sync with e2e/package.json.
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
RUN npx --yes playwright@1.49.1 install --with-deps chromium \
    && chmod -R a+rX /ms-playwright

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
