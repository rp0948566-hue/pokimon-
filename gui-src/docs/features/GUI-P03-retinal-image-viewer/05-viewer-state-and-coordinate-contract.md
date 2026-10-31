# 05 — Viewer State and Coordinate Contract

## Typed Viewer State Structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPanelId {
    Original = 0,
    Lesions = 1,
    Vessels = 2,
    Explainability = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Independent,
    Synchronized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeLaterality {
    OD,          // Right Eye (Oculus Dexter)
    OS,          // Left Eye (Oculus Sinister)
    Unspecified,
}

#[derive(Debug, Clone)]
pub struct ViewerPanelState {
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub hovered_pixel: Option<(u32, u32)>,
    pub hovered_norm: Option<(f32, f32)>,
    pub is_focused: bool,
}
```

## Coordinate Space Transformation Contract

1. **Letterbox Inscription**:
   Given container `viewport: Rect` and raw image dimensions `(W, H)`:
   $$\text{aspect} = \frac{W}{H}$$
   $$\text{container\_aspect} = \frac{\text{viewport.width()}}{\text{viewport.height()}}$$
   If $\text{aspect} > \text{container\_aspect}$, width is constrained to viewport width, and height is centered.
   Otherwise, height is constrained to viewport height, and width is centered.
   This yields base bounding box `fit_base: Rect`.

2. **Pan and Zoom Transformation**:
   $$\text{scaled\_size} = \text{fit\_base.size()} \times \text{zoom}$$
   $$\text{center} = \text{fit\_base.center()} + \text{pan}$$
   $$\text{display\_rect} = \text{Rect::from\_center\_size}(\text{center}, \text{scaled\_size})$$

3. **Screen to Image Mapping**:
   For any mouse pointer $P = (x, y)$:
   ```rust
   if !display_rect.contains(pos) {
       return None; // Over empty letterbox
   }
   let u = ((pos.x - display_rect.min.x) / display_rect.width()).clamp(0.0, 1.0);
   let v = ((pos.y - display_rect.min.y) / display_rect.height()).clamp(0.0, 1.0);
   let px = ((u * img_w).floor() as u32).min((img_w as u32).saturating_sub(1));
   let py = ((v * img_h).floor() as u32).min((img_h as u32).saturating_sub(1));
   Some((pos, (px, py), (u, v)))
   ```
