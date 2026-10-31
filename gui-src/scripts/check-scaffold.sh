#!/usr/bin/env bash
set -e

echo "=== PokéRescue Sentinel Scaffold Validation ==="

echo "1. Checking workspace manifest files..."
test -f package.json || { echo "Missing package.json"; exit 1; }
test -f pnpm-workspace.yaml || { echo "Missing pnpm-workspace.yaml"; exit 1; }
test -f packages/contracts/package.json || { echo "Missing contracts package.json"; exit 1; }
test -f apps/desktop/package.json || { echo "Missing desktop package.json"; exit 1; }
test -f apps/desktop/src-tauri/tauri.conf.json || { echo "Missing tauri.conf.json"; exit 1; }

echo "2. Verifying documentation structure..."
test -f docs/architecture/FRONTEND_ARCHITECTURE.md || { echo "Missing FRONTEND_ARCHITECTURE.md"; exit 1; }
test -f docs/ui/USER_DESIGN_HANDOFF.md || { echo "Missing USER_DESIGN_HANDOFF.md"; exit 1; }
test -f docs/testing/TEST_STRATEGY.md || { echo "Missing TEST_STRATEGY.md"; exit 1; }

echo "Scaffold files verified successfully."
