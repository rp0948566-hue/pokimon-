# PokéRescue Sentinel — Shared Contracts

## Purpose
Pure TypeScript interfaces, enums, and types defining the data contracts between the desktop client frontend, local Tauri IPC, and future backend Rust services.

## Principles
- Zero UI or React dependencies.
- Zero backend implementation logic.
- All timestamps must be ISO-8601 UTC strings.
