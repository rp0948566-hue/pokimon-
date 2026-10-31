# ADR-0002: Tauri 2 Desktop Shell for Windows Packaging

## Status
Accepted (Subject to Windows packaging verification)

## Context
The application targets Windows desktop command centers. We require a lightweight runtime with native installer capabilities and high performance.

## Decision
Adopt Tauri 2 with a Rust desktop shell.
- Drastically lower memory and disk footprint compared to Electron.
- Direct integration path with future local Rust microservices.
- Native Windows installer generation (MSI/EXE).

## Consequences
- Requires Rust build toolchain for packaging.
- Frontend remains web-standards compliant and can run in a browser during UI development.
