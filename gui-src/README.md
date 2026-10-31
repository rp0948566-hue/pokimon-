# PokéRescue Sentinel

Disaster-response command-center desktop application scaffold with a future responder mobile/PWA interface.

## Current Project Status: Scaffold Only
This repository is currently in the **Scaffold Only (No Feature Implementation)** phase. It establishes:
- Enterprise monorepo layout using `pnpm` workspace.
- Strongly-typed contract definitions under `packages/contracts`.
- React 19 + TypeScript + Vite desktop client shell under `apps/desktop`.
- Tauri 2 Windows desktop packaging integration under `apps/desktop/src-tauri`.
- Complete architectural boundaries, test plans, and design handoff specifications.

> [!NOTE]
> The visual design and product logic are intentionally unopinionated placeholders. Final visual design, themes, and design tokens will be supplied by the user.

## Workspace Layout
- `apps/desktop`: Desktop frontend application shell (React + Vite + TypeScript + Tauri 2).
- `packages/contracts`: Strongly-typed data contract definitions shared with future Rust services.
- `docs/`: Comprehensive architecture, product, UI, API, security, and release documentation.

## Prerequisites
- Node.js >= 20 (tested on Node v24.22.1)
- pnpm >= 9 (tested on pnpm 12.15.0)
- Rust toolchain >= 1.80 (for Tauri 2 desktop packaging)

## Quick Start
```bash
# Install workspace dependencies
pnpm install

# Run type check across all packages
pnpm typecheck

# Run unit tests
pnpm test

# Start the desktop web shell in development mode
pnpm dev
```

For detailed architectural and design handoff guidelines, see [docs/README.md](docs/README.md).
