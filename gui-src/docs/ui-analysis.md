# UI Analysis — Pixel-Accurate Frame Recreation

## 1. Project Context & Existing Architecture
- **Workspace Location**: `/home/claude/Desktop/Hackthon dezine`
- **Application Shell**:
  - React 19 + TypeScript + Vite desktop client (`apps/desktop`).
  - Standalone high-performance Rust executable server (`src/main.rs` compiled to `target/release/pokerescue-server`).
- **Entry Points & Shared Layouts**:
  - `apps/desktop/src/layouts/CommandCenterLayout.tsx`: Primary shared layout wrapping all application routes.
  - `apps/desktop/src/layouts/ResponderLayout.tsx`: Secondary layout wrapper for field-responder routes.
  - `apps/desktop/src/styles/index.css`: Global base styles and design-token imports.
  - `apps/desktop/src/styles/design-tokens.css`: Root token definitions.
  - `src/main.rs`: Embedded HTML/CSS canvas served by the standalone Rust server.

---

## 2. Visual Reference Analysis (media_1792996916912_7a526f16.png)

### Overall Proportions & Canvas
- **Viewport Resolution**: `1006 × 780 px`.
- **Page Background (Outer Perimeter)**:
  - Color: `#232323` (solid dark charcoal / smoky charcoal black, RGB `[35, 35, 35]`).
  - Extent: Surrounds the main frame on all four sides (`~44px` horizontal margin, `~82px` vertical margin).

### Main Outer Frame
- **Geometry**: Large horizontal rounded rectangle (`918 × 615 px`, `~91.2%` viewport width, `~78.8%` viewport height).
- **Border**: Heavy solid black stroke (`~5px` thickness, `#000000`).
- **Corner Radius**: `~28px` to `32px` (`border-radius: 28px`).
- **Base Surface**: Light grey (`#D8D8D8`) with an evenly distributed, dense dotted pattern.
- **Dotted Texture Specification**:
  - Dot Color: `#9A9A9A` (darkened for clear optical visibility).
  - Dot Radius: `0.85px`.
  - Grid Pitch: `6px × 6px` Cartesian repetition.
  - Technology: Native CSS `radial-gradient` (zero external images/assets).

### Surrounding Panel Layout (Modular Side & Lower Panels)
1. **Left Side Column** (`width: ~160px`, `~17.5%` frame width):
   - **Upper Panel**: `160 × 395 px` (taller), `border-radius: ~20px`, flat muted warm grey/taupe/mauve (`#C2B3B3`), no heavy border.
   - **Lower Panel**: `160 × 160 px` (smaller square-like), `border-radius: ~16px`, light neutral grey (`#D8D8D8`), matching base canvas tone.
   - **Gap**: `~12px` vertical spacing between upper and lower panels.
2. **Right Side Column** (`width: ~160px`, `~17.5%` frame width):
   - **Upper Panel**: `160 × 395 px` (taller), `border-radius: ~20px`, flat muted warm grey/taupe/mauve (`#C2B6B6`), no heavy border.
   - **Lower Panel**: `160 × 160 px` (smaller square-like), `border-radius: ~16px`, light neutral grey (`#D8D8D8`).
   - **Gap**: `~12px` vertical spacing between upper and lower panels.
3. **Lower Central Panel**:
   - **Dimensions**: `478 × 135 px` (centered horizontally between the side columns).
   - **Border**: `~3.5px` to `4px` solid black stroke (`#000000`).
   - **Corner Radius**: `~16px`.
   - **Fill**: Neutral light grey (`#E0E0E0` / `#D8D8D8`).
   - **Alignment**: Aligned with the bottom tier of side panels, leaving `~16px` vertical clearance above.

---

## 3. Strict Exclusions & Scope Boundaries
- **Upper Central Region Content**: The map screenshot, route line, buildings, shields, and POIs in the reference image are **strictly excluded**.
- **No Map Recreation**: Zero map canvas, Leaflet/Mapbox tiles, or imitation map graphics.
- **Content Insertion Point**: The upper central region (`~560 × 360 px`) remains dedicated to the existing website content / route views.
- **No Unrelated Restyling**: Navigation items, route logic, buttons, APIs, contracts, and backend handlers must remain untouched.

---

## 4. Implementation Constraints
1. **CSS-Only Texture**: Must use pure CSS `radial-gradient` for the dotted pattern, not a static image screenshot.
2. **Responsive Desktop Proportions**: Must preserve desktop visual balance while adapting fluidly via CSS grid/flexbox.
3. **Single Insertion Point**: Frame structure must be applied in the shared root layout (`CommandCenterLayout.tsx` and `src/main.rs`) so all current and future views inherit it without duplication.

---

## 5. Root Cause of Sizing Issue & Full-Screen Analysis
- **Root Cause**:
  1. `.main-frame` had rigid caps: `max-width: 1006px` and `max-height: 720px`. On standard 1080p, 1440p, or 4K screens, this forced the application to render as a miniature card centered inside massive dark empty borders.
  2. `.page-backdrop` applied `align-items: center` and `justify-content: center` with large `24px` padding, causing excessive perimeter margins.
- **Required Resolution**:
  1. Remove `max-width: 1006px` and `max-height: 720px` from `.main-frame` so it expands to `100%` of the available application viewport.
  2. Set `.page-backdrop` to fill `100vw` and `100dvh` (with `100vh` fallback).
  3. Ensure `.side-column` uses fluid clamping (`clamp(160px, 14vw, 220px)`) and `.panel-lower-center` uses fluid horizontal sizing (`clamp(480px, 50vw, 720px)`) to keep all panels proportionally balanced without distortion across screen sizes.
  4. Preserve all existing elements, borders, textures, colors, navigation, and children without removing or replacing anything.

---

## 6. Outer Frame Black Fill Removal & Pink Rectangle Analysis
- **Root Cause of Unwanted Black Region**:
  1. `html, body` and `.page-backdrop` used `background-color: var(--color-page-bg, #232323)`, painting a solid dark charcoal/black fill around the interface.
  2. `.page-backdrop` had `padding: 8px` which created an 8px dark band around `.main-frame`.
  3. `.main-frame` had `box-shadow: 0 10px 40px rgba(0, 0, 0, 0.6)` casting an opaque black shadow perimeter.
- **Resolution for Outer Frame**:
  1. Remove the black fill by changing `--color-page-bg` and `.page-backdrop` background to `transparent` (or matching light substrate).
  2. Eliminate the perimeter padding (`padding: 0`) and remove the heavy black `box-shadow`.
  3. **Preserve Outer Black Border**: `border: var(--border-frame, 5px solid #000000)` and `border-radius: var(--radius-frame, 24px)` remain strictly intact as independent border properties.
  4. **Preserve Dotted Texture**: The light grey dotted canvas (`#D8D8D8` with `#9A9A9A` dots) on `.main-frame` remains unchanged.
- **Specification for Pink Rounded Rectangle (Reference Image 3)**:
  1. **Element**: `.pink-panel` residing inside `.center-content-slot`.
  2. **Fill Color**: `#C78F90` (solid muted salmon-pink, no gradient, no texture inside).
  3. **Corners**: Smooth rounded corners with `border-radius: 22px` (matching the 18px–24px specification).
  4. **Border**: `none` (clean edges, zero black outline).
  5. **Dimensions & Positioning**: `width: 96%`, `height: 92%`, centered via `margin: auto` within `.center-content-slot`, allowing the surrounding light grey dotted texture to be visible around its perimeter.
  6. **Content Preservation**: Hosts the existing website navigation and workspace views seamlessly without breaking route switching or user interactions.
