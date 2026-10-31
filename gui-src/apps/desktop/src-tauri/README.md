# Tauri Desktop Shell for PokéRescue Sentinel

## Purpose
Native desktop host process written in Rust, responsible for window management, lifecycle events, and future native service bridges on Windows.

## Directory Structure
- `Cargo.toml`: Rust package manifest for the desktop host.
- `build.rs`: Tauri 2 build script.
- `tauri.conf.json`: Tauri 2 application and packaging configuration.
- `capabilities/default.json`: Least-privilege capability definitions.
- `src/main.rs`: Windows executable entry point.
- `src/lib.rs`: Tauri app runtime setup.
- `icons/`: Desktop application icons awaiting user branding.
