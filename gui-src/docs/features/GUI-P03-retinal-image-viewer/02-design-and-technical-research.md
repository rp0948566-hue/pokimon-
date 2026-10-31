# 02 — Design and Technical Research

## Medical Imaging Ergonomics & egui Graphics

### 1. Viewport Geometry and Aspect Fit
Medical fundus photographs have fixed sensor aspect ratios (typically 4:3, 3:2, or 1:1 circular masks). Distorting this aspect ratio introduces clinical misinterpretation.
- `aspect_fit(img_w, img_h, container)` calculates the maximum inscribed rectangle centered within the container viewport.
- Unused viewport space constitutes the letterbox margin.
- Coordinates reported to clinicians must strictly lie within the active image content rectangle, never within the black letterbox bars.

### 2. Cursor-Centered Zoom Mathematics
Standard center-zoom forces the clinician to pan continuously while zooming in on peripheral retinal lesions. Cursor-centered zoom preserves the anatomical point under the mouse cursor:
$$\text{mouse\_offset} = \text{mouse\_pos} - \text{center}$$
$$\text{zoom\_ratio} = \frac{\text{new\_zoom}}{\text{old\_zoom}}$$
$$\text{new\_center} = \text{mouse\_pos} - \text{mouse\_offset} \times \text{zoom\_ratio}$$
$$\text{new\_pan} = \text{new\_center} - \text{fit\_base.center()}$$

### 3. Screen-to-Image Coordinate Transformation
Given a screen coordinate $P_{\text{screen}} = (x, y)$ inside the panel:
1. Verify $P_{\text{screen}} \in \text{display\_rect}$. If false, return `None` (cursor is over background/letterbox).
2. Calculate normalized coordinates:
   $$u = \frac{x - \text{display\_rect.min.x}}{\text{display\_rect.width()}} \in [0.0, 1.0]$$
   $$v = \frac{y - \text{display\_rect.min.y}}{\text{display\_rect.height()}} \in [0.0, 1.0]$$
3. Map to discrete image pixel space:
   $$X_{\text{pixel}} = \lfloor u \times W_{\text{image}} \rfloor$$
   $$Y_{\text{pixel}} = \lfloor v \times H_{\text{image}} \rfloor$$
   Clamp $X_{\text{pixel}} \in [0, W - 1]$ and $Y_{\text{pixel}} \in [0, H - 1]$.

### 4. Multi-Panel Synchronization Strategy
When examining lesions across multiple representations (e.g. comparing raw fundus with vascular segmentation), clinicians need synchronized viewports:
- **Independent Mode**: Each viewport maintains its own `zoom` and `pan`.
- **Synchronized Mode**: Pan and zoom events on any viewport immediately propagate to all viewports displaying registered artifacts of the same examination.

### 5. Color Palette & Visual Token Hierarchy
- Viewport Surface: Deep charcoal `#101114` (neutral, non-distracting background for fundus examination).
- Panel Border: Subtle neutral stroke `#2E3035` (unselected), `#3B82F6` (active / focused).
- Lesion Overlay Colors:
  - Microaneurysms: Vivid Red `#EF4444`
  - Hemorrhages: Deep Crimson `#DC2626`
  - Hard Exudates: Medical Amber/Yellow `#F59E0B`
  - Cotton-Wool Spots: Cyan/White `#06B6D4`
- Vessel Segmentation: Emerald Green `#10B981` / High-contrast White `#F9FAFB`
- Explainability (Grad-CAM): Warm Heatmap gradient (Blue -> Green -> Yellow -> Red)
