# ADR-0005: Pure Rust Architecture and Executable Server

## Status
Accepted (Supersedes ADR-0001 and ADR-0002)

## Context
The user directed an architectural pivot: discontinue the multi-layer Node.js / TypeScript / Vite / Tauri monorepo stack in favor of building the entire system **strictly in the Rust programming language** as a single executable server (`.exe` on Windows / binary on Linux).

## Decision
1. **Language Standardization**: The entire codebase (server, domain models, static asset delivery, and API services) is consolidated 100% into Rust.
2. **Runtime Model**: The application builds into a single self-contained native executable binary that runs as a local HTTP / WebSocket server.
3. **Asset Delivery**: Static web assets (including the analyzed static dot-grid canvas) are embedded directly into the Rust binary or served via local routes, eliminating external runtime dependencies (Node.js, pnpm, npm).
4. **Desktop Execution**: On Windows, the binary compiles to a standalone `.exe`. When launched, it binds to a local port and serves the interface and command services.

## Consequences
- **Positive**: Zero Node.js runtime footprint, instantaneous startup, minimal memory consumption, unified type safety across backend and endpoints.
- **Maintenance**: No synchronization needed between TypeScript contracts and Rust structs; Rust types serve as the single source of truth.
