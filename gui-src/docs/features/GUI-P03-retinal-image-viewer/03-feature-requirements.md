# 03 — Feature Requirements: Retinal Image Viewer & Quad-Panel Workspace

## 1. Functional Requirements

### 1.1 Quad-Panel Layout
- **FR-01**: Report page displays a 2x2 grid of equal-priority diagnostic panels:
  - Panel 0: Original Fundus Image
  - Panel 1: AI Lesion Detection
  - Panel 2: Vessel Analysis
  - Panel 3: AI Explainability (Grad-CAM)
- **FR-02**: Wide window mode displays a 2x2 grid; smaller window widths gracefully scroll or adjust without clipping critical controls or distorting image aspect ratios.
- **FR-03**: Any panel can be maximized/expanded to fill the entire workspace via a maximize button or keyboard shortcut, and restored via Escape or restore button.

### 1.2 Original Image Presentation
- **FR-04**: Load and display local fundus photographs (PNG, JPG, JPEG, WEBP, BMP) using safe metadata scrubbing and bounded memory.
- **FR-05**: Display image resolution (`W x H`), file size, safe filename, and examination eye laterality (`OD` - Right Eye / `OS` - Left Eye).
- **FR-06**: Eye laterality is an explicit operator toggle/metadata, never inferred blindly from visual features.

### 1.3 Interactive Pan and Zoom
- **FR-07**: Mouse wheel smoothly zooms in/out with cursor-centered focal point (zoom clamped between 0.25x and 10.0x).
- **FR-08**: HUD toolbar provides `[−]`, `[+]`, `[Fit]`, and `[↺ Reset]` buttons.
- **FR-09**: Click-and-drag pans the image smoothly within the viewport.
- **FR-10**: Double-click or reset button restores default fit-to-view zoom and zero pan offset.

### 1.4 Coordinate Inspection HUD
- **FR-11**: Track cursor position when hovering over active image pixels.
- **FR-12**: Display exact discrete pixel coordinates `(X, Y)` and normalized coordinates `(u, v) ∈ [0.0, 1.0]`.
- **FR-13**: Suppress coordinate readout when hovering over letterbox margins or when no image is loaded.

### 1.5 Multi-Panel Synchronization
- **FR-14**: Workspace header provides `[⚡ Synchronized]` vs `[⛌ Independent]` mode toggle.
- **FR-15**: In Synchronized mode, zoom and pan actions on any panel propagate across all active panels.
- **FR-16**: In Independent mode, each panel retains its private zoom and pan coordinates.

### 1.6 AI Overlay Presentation & Truthful States
- **FR-17**: Panel 1 (Lesions) supports category visibility toggles (Microaneurysms, Hemorrhages, Hard Exudates, Cotton-wool spots) and overlay opacity slider (0% to 100%).
- **FR-18**: Panel 2 (Vessels) supports overlay opacity slider and "Overlay" vs "Mask Only" mode toggle.
- **FR-19**: Panel 3 (Explainability) supports heatmap opacity slider and attribution model/version label.
- **FR-20**: When inference artifacts are absent from the backend, Panels 1, 2, and 3 display truthful, informative "Awaiting Freebuff Backend" cards. Zero fabricated diagnoses or fake lesion markers.

## 2. Non-Functional Requirements
- **NFR-01**: 100% native Rust using `egui` and `eframe`. Zero web runtimes or HTML/CSS/JS.
- **NFR-02**: Zero modifications to Codex's backend files in `native/crates/`.
- **NFR-03**: Smooth 60 FPS viewport rendering; decoded textures cached in application state.
- **NFR-04**: Strict privacy protection: scrub EXIF metadata, never transmit images over external networks.
