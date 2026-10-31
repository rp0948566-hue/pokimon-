# 07 — Testing Strategy: Quad-Panel Workspace

## Test Layers
1. **Mathematical Coordinate Transform Tests**:
   - `test_aspect_fit_wide_vs_tall`: Verify non-distorting bounding box calculation.
   - `test_screen_to_image_coords_centered`: Verify exact pixel index computation for known container rects.
   - `test_screen_to_image_coords_rejects_letterbox`: Verify that points outside `display_rect` return `None`.
   - `test_cursor_centered_zoom`: Verify that the focal pixel coordinate remains identical before and after zoom scaling.
   - `test_zoom_clamping`: Verify zoom stays bounded between 0.25x and 10.0x.
   - `test_pan_reset`: Verify reset restores `zoom = 1.0` and `pan = Vec2::ZERO`.
2. **State & Synchronization Tests**:
   - `test_independent_mode_isolation`: Verify zooming in Panel 0 does not alter Panel 1 zoom in Independent mode.
   - `test_synchronized_mode_propagation`: Verify pan and zoom in one panel propagates to all active panels in Synchronized mode.
   - `test_fullscreen_toggle_and_escape`: Verify expanding Panel 2 sets `expanded_panel = Some(ViewPanelId::Vessels)` and reset clears it to `None`.
   - `test_lesion_category_toggles`: Verify toggling individual lesion categories modifies visibility state.
   - `test_vessel_mask_only_toggle`: Verify switching between overlay and mask-only mode.
   - `test_eye_laterality_toggle`: Verify toggling OD to OS updates metadata without re-inferring image content.
3. **Automated Static Verification**:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `cargo build --release`
4. **Live GUI Verification**:
   - Launch native desktop executable with `WAYLAND_DISPLAY="" DISPLAY=:0`.
   - Test mouse drag, wheel zoom, double-click, buttons, HUD readout, and responsive resizing.
   - Capture 6 required high-resolution desktop verification PNGs.
