# Dev container for wholesome-swolesome.
# Provides: Rust (stable) + wasm32 target, Trunk, wasm-pack,
# Node + Claude Code CLI, Playwright with Chromium.
#
# Runs as non-root `node` user so `claude --dangerously-skip-permissions`
# is permitted (the CLI refuses to use that flag as root).

FROM node:22-bookworm

# System deps:
#   - build-essential, pkg-config, libssl-dev: building Rust crates with C deps
#   - git, curl, ca-certificates: general dev
#   - Playwright Chromium runtime deps (fonts, libs)
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential pkg-config libssl-dev \
      git curl ca-certificates \
      libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libcups2 \
      libdrm2 libdbus-1-3 libxkbcommon0 libxcomposite1 libxdamage1 \
      libxfixes3 libxrandr2 libgbm1 libpango-1.0-0 libcairo2 libasound2 \
      fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

# Global npm packages installed as root before dropping privileges.
RUN npm install -g @anthropic-ai/claude-code

USER node
WORKDIR /home/node

# Rust toolchain via rustup, pinned to stable. Add wasm target up front.
ENV RUSTUP_HOME=/home/node/.rustup \
    CARGO_HOME=/home/node/.cargo \
    PATH=/home/node/.cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup target add wasm32-unknown-unknown

# Trunk (WASM bundler/dev-server) + wasm-pack (used by the test suite).
# Use --locked for reproducible builds.
RUN cargo install --locked trunk wasm-bindgen-cli wasm-pack

# Playwright browser (Chromium only — Firefox/Webkit not needed here).
# Browsers land in /home/node/.cache/ms-playwright.
RUN npx --yes playwright@1 install chromium

WORKDIR /workspace
EXPOSE 8080

CMD ["bash"]
