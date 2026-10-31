# GUI Phase 1 • Task 03 — Retinal Image Viewer and Quad-Panel Report Workspace

## Overview
RetinaCare GUI Phase 1 Task 03 upgrades the Report page from a single-stage preview into a fully interactive, clinically organized four-panel retinal visualization workspace implemented in 100% native Rust using `egui` and `eframe`.

## Key Capabilities
1. **Four-Panel Analysis Grid**:
   - Panel 0: Original Fundus Image (authentic image display, aspect-fit, 100% actual pixel mode, metadata).
   - Panel 1: AI Lesion Detection (overlay rendering, category toggles for Microaneurysms, Hemorrhages, Hard Exudates, Cotton-wool spots, opacity slider).
   - Panel 2: Vessel Analysis (vascular segmentation overlay, mask-only toggle, opacity slider).
   - Panel 3: AI Explainability (Grad-CAM / attribution heatmap blending, opacity slider, attribution legend).
2. **Interactive Viewport Operations**:
   - Cursor-centered mouse-wheel zoom and toolbar controls (`+`, `-`, fit, reset).
   - Drag-to-pan with smooth clamped bounds.
   - Panel expansion / fullscreen with Escape or maximize toggle.
3. **Coordinate Inspection**:
   - Screen-to-image pixel mapping accounting for letterbox offsets, zoom, and pan.
   - Normalized coordinate readout `(u, v) ∈ [0.0, 1.0]` and exact pixel coordinates `(X, Y)`.
   - Hover detection clamped to valid image bounds (no coordinates over black letterbox bars).
4. **Multi-Panel Synchronization**:
   - Independent View: Each panel maintains its own pan and zoom.
   - Synchronized View: Synchronizes pan and zoom across all panels displaying aligned examination artifacts.
5. **Truthful Availability States**:
   - Panels 1, 2, and 3 display truthful "Awaiting Freebuff Backend" cards when inference outputs are absent.
   - Zero fabricated diagnoses, zero hallucinated lesion coordinates, zero fake heatmaps.

## Architecture Documents
- [01-existing-viewer-audit.md](01-existing-viewer-audit.md)
- [02-design-and-technical-research.md](02-design-and-technical-research.md)
- [03-feature-requirements.md](03-feature-requirements.md)
- [04-implementation-plan.md](04-implementation-plan.md)
- [05-viewer-state-and-coordinate-contract.md](05-viewer-state-and-coordinate-contract.md)
- [06-backend-artifact-contract.md](06-backend-artifact-contract.md)
- [07-testing-strategy.md](07-testing-strategy.md)
- [08-functional-test-results.md](08-functional-test-results.md)
- [09-visual-verification.md](09-visual-verification.md)
- [10-file-change-report.md](10-file-change-report.md)
- [11-known-limitations.md](11-known-limitations.md)
- [12-task-04-handoff.md](12-task-04-handoff.md)
