# Phase 1 • Task 06.1: Dedicated Patient Screens & Navigation Redesign

## 1. Overview
This task corrects the GUI navigation architecture and screen layouts identified in baseline audit screenshots:
- Decouples viewport manipulation tools from clinical report reviewing and document exporting.
- Replaces sparse, accordion-only patient listings with a responsive master-detail desktop layout ($\ge 900\text{px}$) and dedicated full-view screens.
- Adds typed subroutes (`PatientSubroute` and `ReportSubroute`), persistent breadcrumbs, and explicit `← Back` actions.
- Preserves 100% of existing functionality, Stage I AI inference integration, sensitive unencrypted JSON security warnings, and truthful backend unavailable states.

---

## 2. Document Map
- `01-audit.md`: Analysis of Screenshot A (overcrowded toolbar) and Screenshot B (sparse patient canvas).
- `02-navigation-route-diagram.md`: Hierarchical routing state diagram for top-level pages and subroutes.
- `03-screen-requirements.md`: Detailed functional and visual specifications for all 5 redesigned views.
- `04-implementation-plan.md`: Step-by-step execution plan and verification strategy.
- `05-test-results.md`: Complete automated test execution summary (109 passed).
- `06-desktop-verification.md`: Native X11 screenshot gallery across all 7 views.
- `07-file-change-report.md`: Code-level breakdown of edits in `domain.rs`, `patients.rs`, `viewer.rs`, and `main.rs`.
- `08-known-blockers.md`: Truthful disclosure of missing Codex backend crates (`retinacare-reports`, `audit_events`).
- `09-task07-handoff.md`: Typed API and schema requirements for Phase 2 integration.
