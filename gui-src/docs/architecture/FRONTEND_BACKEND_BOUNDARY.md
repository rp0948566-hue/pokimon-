# Frontend / Backend Boundary

## Boundary Rules
1. **Contract Independence**: Frontend interfaces never directly invoke native OS APIs or Rust crates. All native communication routes through documented Tauri IPC commands (`@tauri-apps/api/core`) or standard HTTP endpoints.
2. **Untrusted Client Principle**: The frontend is treated as an untrusted presentation client. Critical safety rules (e.g., road passability, responder dispatch approvals) must be validated and signed by the backend service.
3. **Data Freshness and Timestamps**: All machine contracts require ISO-8601 UTC timestamps (`observed_at`, `received_at`). Timezone conversions occur strictly at the presentation layer.
4. **Transport Decoupling**: The UI layer consumes abstract services (`IncidentService`, `RouteService`). The underlying transport (REST, local Tauri IPC, or offline SQLite) is swappable without altering UI components.
