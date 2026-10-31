# Planned Component Inventory

Candidate shared components awaiting visual design approval:

## UI Primitives (`components/ui/`)
- `Button`: Primary, secondary, danger, ghost variants.
- `Badge`: Status tags (Passable, Blocked, Unverified, Stale).
- `Card`: Surface container with border and elevation tokens.
- `Modal / Dialog`: Overlay window for confirmations and detailed triage.

## Layout Components (`components/layout/`)
- `SidebarNavigation`: Main navigation menu for desktop command center.
- `TopBar`: Incident title, connectivity badge, active user profile.
- `SplitView`: Resizable dual-pane container (e.g. Map + Incident Details).

## Domain Displays (`components/status/`, `components/data-display/`)
- `IncidentSeverityBadge`: Visual indicator for critical triage levels.
- `FreshnessIndicator`: Indicator for data age (Fresh, Approaching Stale, Stale).
- `TelemetryFeed`: Live streaming sensor and vehicle telemetry rows.
