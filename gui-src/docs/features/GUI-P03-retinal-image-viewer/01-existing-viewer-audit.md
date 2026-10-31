# 01 — Existing Viewer Audit

## Audit Date
2026-10-28

## Current Implementation State
Prior to Task 03, the Report page (`AppPage::Report` in `rust/src/main.rs`) had the following structure:
1. **Layout**:
   - Single large `center_box_rect` on the left.
   - Right-side card `right_card_rect` (280 px wide) containing 4 small thumbnail tiles (`MODES`: Original Image, AI Lesion Detection, Vessel Analysis, Grad-CAM).
   - Only Box 0 displayed the uploaded image. Boxes 1, 2, 3 were static empty placeholders with "Empty" labels.
   - Selecting a tile changed `self.active_mode`, but no genuine multi-panel comparison or overlays existed.
2. **Viewer State**:
   - `ImageViewerState` in `rust/src/ui/components.rs` had only `zoom: f32` and `pan: Vec2`.
   - Single global zoom and pan state shared across all modes.
   - Zoom was center-scaled rather than cursor-centered.
   - No screen-to-image pixel coordinate mapping or hovering inspection.
   - No panel maximization / fullscreen mode.
   - No synchronization toggle between panels.
3. **Image Ingestion**:
   - `pick_and_load_image` and `load_fundus_from_path` safely validate paths, scrub metadata, decode with `image::load_from_memory`, and upload to `egui::ColorImage`.
   - Dimensions (`uploaded_dimensions`) and filename (`uploaded_filename`) are stored in `RetinaCareApp`.
4. **Overlay & Backend Contracts**:
   - No domain contracts existed for lesion masks, vessel segmentations, or explainability heatmaps.
   - No category toggles (microaneurysms, hemorrhages, hard exudates, cotton-wool spots).
   - No opacity sliders.

## Audit Summary by Component
| Component | Existing Status | Action in Task 03 |
|---|---|---|
| Quad-Panel 2x2 Workspace | PARTIAL (Single stage + thumbnails) | Rebuild as true 2x2 interactive grid |
| Original Image Display | IMPLEMENTED_NEEDS_IMPROVEMENT | Add 1:1 pixel mode, laterality toggle, coordinate HUD |
| Pan & Zoom Mechanics | IMPLEMENTED_NEEDS_IMPROVEMENT | Upgrade to cursor-centered zoom and clamped pan |
| Coordinate Inspection | NOT_IMPLEMENTED | Implement exact mathematical screen-to-pixel transform |
| Multi-Panel Sync | NOT_IMPLEMENTED | Add Independent vs Synchronized mode toggle |
| Panel Maximization | NOT_IMPLEMENTED | Add Fullscreen expand/restore with Escape shortcut |
| Lesion Overlay View | NOT_IMPLEMENTED | Add category toggles, opacity, truthful awaiting state |
| Vessel Analysis View | NOT_IMPLEMENTED | Add mask/overlay toggle, opacity, truthful awaiting state |
| Explainability View | NOT_IMPLEMENTED | Add heatmap blending, opacity, truthful awaiting state |
| Backend Artifact Contract | NOT_IMPLEMENTED | Define typed Rust structures in `domain.rs` |
