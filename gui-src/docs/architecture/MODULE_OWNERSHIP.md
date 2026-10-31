# Module Ownership and Directory Responsibilities

| Path | Primary Responsibility | Allowed Dependencies | Prohibited Content |
| :--- | :--- | :--- | :--- |
| `apps/desktop/src/app` | Application bootstrap, routing, root providers | Features, contracts, layouts | Direct business logic |
| `apps/desktop/src/layouts` | Structural shell layouts (Command Center, Responder) | Shared UI components, router outlet | Direct API fetch logic |
| `apps/desktop/src/pages` | Route destination view components | Corresponding feature module | Unrelated feature components |
| `apps/desktop/src/features` | Self-contained domain UI and local state | Shared components, contracts, lib | Other feature internals |
| `apps/desktop/src/components` | Reusable visual primitives and layouts | Design tokens, utils | Feature domain logic |
| `apps/desktop/src/contracts` | Local re-exports and adapter types | `@pokerescue/contracts` | Runtime implementation logic |
| `apps/desktop/src/services` | Communication adapters (HTTP, Tauri, WS) | Contracts, platform lib | UI components |
| `packages/contracts` | Pure TypeScript interfaces and types | Zero frontend or backend deps | Implementation code |
| `apps/desktop/src-tauri` | Rust desktop host process | Tauri core crates | Web application state |
