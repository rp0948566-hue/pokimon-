# PokéRescue Sentinel — Frontend & Desktop Scaffold Plan

## Current State Findings
- **Workspace Location**: `/home/claude/Desktop/Hackthon dezine`
- **Existing User Work**:
  - `documents/dotes adn backgroudn colour/` containing image analysis artifacts.
  - `ponytail/` skill repository.
  - Initial `docs/` and `memory/` stubs.
  - Agent rule files (`CLAUDE.md`, `GEMINI.md`, `AGENTS.md`, `Ghost.md`).
- **Toolchain Environment**:
  - Node.js: `v24.22.1`
  - npm: `11.19.1`
  - pnpm: `12.15.0`
  - Rust: `1.99.1`
  - Cargo: `1.99.1`
  - Operating System: Linux x86_64

## Scaffold Objectives
1. Build an enterprise-grade monorepo structure with `pnpm` workspace.
2. Establish `apps/desktop` powered by React 19 / TypeScript / Vite / Tauri 2 shell.
3. Establish `packages/contracts` for strongly-typed communication contracts without embedding backend logic.
4. Establish clean architectural boundaries: zero visual design invention, zero mock backend logic, zero live sensor simulations in this phase.
5. Create complete documentation manifests, testing strategy, accessibility checklists, Windows packaging roadmap, and design handoff interfaces.

## Execution Sequence
1. Workspace root configuration (`package.json`, `pnpm-workspace.yaml`, `.editorconfig`, `.gitattributes`, `.gitignore`, `.npmrc`, `.env.example`).
2. Core project governance and legal documents (`README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`, `LICENSE`).
3. Complete documentation manifest across `docs/product`, `docs/architecture`, `docs/decisions`, `docs/ui`, `docs/api`, `docs/security`, `docs/privacy`, `docs/testing`, `docs/release`, `docs/roadmap`.
4. `packages/contracts` shared contract library.
5. `apps/desktop` React + Vite + TypeScript application shell and feature/page folder hierarchy.
6. `apps/desktop/src-tauri` desktop shell configuration and minimal Rust entrypoint.
7. Validation suite execution and recording in `SCAFFOLD_VALIDATION_REPORT.md`.
