# 09 — Visual Verification: Retinal Image Viewer

## Verification Protocol
All screenshots are captured from the actual compiled native Linux desktop binary (`rust/target/release/retinacare`) executing on an X11 / Xwayland surface.

## Required Verification Artifacts

### 1. `report_empty_verified.png`
- **Condition**: Initial navigation to Report page before image upload.
- **Visual Evidence**:
  - Top toolbar displays "Retinal Analysis Workspace" with OD laterality pill and Synchronized mode toggle.
  - Panel 0 shows clean "No Retinal Image Loaded" guidance card.
  - Panels 1, 2, and 3 display truthful "Awaiting Backend Output" clinical notices with zero fake data.
  - Bottom HUD displays "⌖ Awaiting image".

### 2. `report_original_image_verified.png`
- **Condition**: Retinal image loaded into Panel 0 (`synthetic_fundus_od.png`).
- **Visual Evidence**:
  - Authentic non-distorted aspect-fitted fundus scan in Panel 0.
  - Top toolbar displays safe filename and dimensions `1024x1024`.
  - Header displays `OD` laterality badge and zoom controls.

### 3. `report_quad_panel_verified.png`
- **Condition**: Standard balanced 2x2 quad-panel layout.
- **Visual Evidence**:
  - Panel 0 (Original), Panel 1 (Lesions), Panel 2 (Vessels), Panel 3 (Explainability) rendered with equal visual priority.
  - Lesion category pills (`MA`, `HE`, `EX`, `CWS`) and opacity controls.
  - Vessel mode toggle (`Overlay` / `Mask Only`).
  - Explainability attribution tag.

### 4. `report_zoom_pan_verified.png`
- **Condition**: Zoomed (150%) and panned view with coordinate inspection active.
- **Visual Evidence**:
  - Image smoothly scaled with cursor-centered focal point.
  - Bottom HUD displays exact pixel coordinate readout `⌖ X: 512 Y: 512 • u: 0.500 v: 0.500`.
  - Zoom indicator reflects `150%`.

### 5. `report_backend_unavailable_verified.png`
- **Condition**: Expanded / focused inspection of AI diagnostic panels.
- **Visual Evidence**:
  - Prominent "Awaiting Validated Lesion Inference" / "Awaiting Vessel Segmentation" status cards.
  - Explicit notification that AI models (`HSMoE-AUNet`, `SwinHRUNet++`, `HierarchiRetina`) are offline.
  - Zero synthetic lesion markers or decorative heatmaps.

### 6. `report_small_window_verified.png`
- **Condition**: Narrow window size (<720 px).
- **Visual Evidence**:
  - Vertical responsive layout stacking panels cleanly with a dedicated vertical scroll area.
  - Zero clipping of toolbar buttons, headers, or zoom controls.
  - Non-distorting aspect fit preserved across all viewports.
