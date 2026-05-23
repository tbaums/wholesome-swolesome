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
      git curl ca-certificates gnupg \
      libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libcups2 \
      libdrm2 libdbus-1-3 libxkbcommon0 libxcomposite1 libxdamage1 \
      libxfixes3 libxrandr2 libgbm1 libpango-1.0-0 libcairo2 libasound2 \
      fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

# GitHub CLI from the official cli.github.com apt repo (Debian's `gh` is stale).
RUN install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
        | gpg --dearmor -o /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && chmod 0644 /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
        > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*

# Global npm packages installed as root before dropping privileges.
RUN npm install -g @anthropic-ai/claude-code

# Pre-create the Claude config dir as `node` so the named volume that
# mounts here in docker-compose inherits correct ownership/perms, and the
# nested `.credentials.json` bind mount has a valid parent.
RUN install -d -o node -g node -m 0700 /home/node/.claude

# Login shells on Debian (`bash -l`, `bash --login`) run /etc/profile, which
# re-sets PATH from /etc/login.defs and clobbers the image's ENV PATH. Drop
# a profile.d snippet so cargo bins and ~/.local/bin survive a login shell.
RUN printf '%s\n' \
      'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"' \
      > /etc/profile.d/10-devcontainer-path.sh \
    && chmod 0644 /etc/profile.d/10-devcontainer-path.sh

USER node
WORKDIR /home/node

# Rust toolchain via rustup, pinned to stable. Add wasm target up front.
# Include ~/.local/bin so any user-installed CLIs are on PATH.
ENV RUSTUP_HOME=/home/node/.rustup \
    CARGO_HOME=/home/node/.cargo \
    PATH=/home/node/.local/bin:/home/node/.cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup target add wasm32-unknown-unknown

# Trunk (WASM bundler/dev-server) + wasm-pack (used by the test suite).
# Use --locked for reproducible builds.
RUN cargo install --locked trunk wasm-bindgen-cli wasm-pack

# Playwright browser (Chromium only — Firefox/Webkit not needed here).
# Browsers land in /home/node/.cache/ms-playwright.
RUN npx --yes playwright@1 install chromium

# Seed a baseline Claude settings file so the CLI doesn't prompt for the
# "dangerous mode" permission on every invocation. The named volume that
# backs ~/.claude in docker-compose only inherits this on first creation.
RUN printf '%s\n' \
      '{' \
      '  "skipDangerousModePermissionPrompt": true,' \
      '  "skipAutoPermissionPrompt": true,' \
      '  "theme": "dark"' \
      '}' > /home/node/.claude/settings.json

# Entrypoint sets up the persistent ~/.claude.json symlink before handing
# off to the container's main command. `docker exec` calls bypass this.
COPY --chown=node:node entrypoint.sh /usr/local/bin/devcontainer-entrypoint
USER root
RUN chmod 0755 /usr/local/bin/devcontainer-entrypoint
USER node

WORKDIR /workspace
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/devcontainer-entrypoint"]
CMD ["sleep", "infinity"]
