# 04 — Implementation Plan: Quad-Panel Workspace

## Phase Plan

### Step 1: Domain & Contract Layer (`rust/src/domain.rs`)
1. Define `EyeLaterality` (`OD`, `OS`, `Unspecified`).
2. Define `ViewPanelId` (`Original`, `Lesions`, `Vessels`, `Explainability`).
3. Define `SyncMode` (`Independent`, `Synchronized`).
4. Define `ArtifactLoadStatus` (`NoImageSelected`, `ImageReady`, `ArtifactUnavailable`, `ArtifactReady`, `ArtifactInvalid`, `AnalysisFailed`).
5. Define `LesionCategory` and `LesionCategoryToggles`.
6. Define artifact records (`LesionMaskArtifact`, `VesselMaskArtifact`, `ExplainabilityArtifact`).
7. Define `ViewerPanelState` and `RetinalExaminationView`.

### Step 2: Viewer Math & Component Layer (`rust/src/ui/viewer.rs`)
1. Implement coordinate math:
   - `compute_display_rect(viewport, img_w, img_h, zoom, pan) -> (Rect, Rect)`
   - `screen_to_image_coords(pos, display_rect, img_w, img_h) -> Option<(Pos2, (u32, u32), (f32, f32))>`
   - `cursor_centered_zoom(mouse_pos, zoom_mult, current_zoom, current_pan, fit_base, min_zoom, max_zoom) -> (f32, Vec2)`
2. Implement panel viewport rendering:
   - Base image rendering with clip rect.
   - Overlay blending for lesion masks, vessel masks, and explainability heatmaps.
   - Crosshair indicator at hovered image point.
   - Header with panel title, status badge, mode toggles, and maximize button.
   - Bottom HUD with coordinate inspection readout and zoom controls.
   - Informative "Awaiting Freebuff Inference" card when artifact is unavailable.
3. Implement 2x2 grid layout calculation and single-panel expanded layout calculation.

### Step 3: Application State & Event Wiring (`rust/src/main.rs`)
1. Integrate `RetinalExaminationView` and panel state inside `RetinaCareApp`.
2. Connect workspace toolbar: Upload button, Laterality selector (`OD / OS`), Sync toggle (`Synchronized / Independent`), Reset All.
3. Wire keyboard shortcuts: Escape restores expanded panel to quad grid; `+` / `-` zooms focused viewport.
4. Support drag-to-pan, mouse-wheel zoom, double-click reset across panels.
5. Propagate pan/zoom in Synchronized mode to all panels.

### Step 4: Verification & Automated Tests
1. Unit tests for coordinate math, letterbox margin rejection, zoom clamping, reset, sync propagation, category toggles.
2. Formatter and linter checks (`cargo fmt`, `cargo clippy`).
3. Native application launch and screenshot captures for all 6 required states.
