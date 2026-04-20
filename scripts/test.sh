#!/bin/bash
set -e

# Run both the WASM unit tests and Playwright E2E tests.
# Prerequisites: wasm-pack (cargo install wasm-pack), Node.js, npx playwright install

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
