#!/bin/bash
set -e

# Run both the WASM unit tests and Playwright E2E tests.
# Prerequisites:
#   cargo install wasm-pack
#   brew install node
#   npm install  (in repo root)
#   npx playwright install webkit

# Ensure ~/.cargo/bin is on PATH (for wasm-pack and trunk)
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> WASM unit tests (csv parsing)"
wasm-pack test --headless --chrome --lib

echo ""
echo "==> Installing Playwright browsers (no-op if already installed)"
npx playwright install --with-deps webkit

echo ""
echo "==> Playwright E2E tests (iPhone 15 / WebKit)"
npx playwright test

echo ""
echo "All tests passed ✓"
