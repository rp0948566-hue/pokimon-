# Dot Structure and Behavior Analysis

## 1. Overview
Structural analysis of the micro-dot matrix pattern across the canvas, specifying physical morphology, total dot census, spatial presence across rows and columns, and strict operational behavior rules.

---

## 2. Dot Matrix Census

| Parameter | Measured Count | Description |
| :--- | :--- | :--- |
| **Total Visible Dots** | 518 dots detected | Uniform coverage across entire sampled frame |
| **Grid Rows (Y-Axis)** | 16 discrete rows | Parallel horizontal lines spanning top to bottom |
| **Grid Columns (X-Axis)** | ~35 columns | Parallel vertical lines spanning left to right |
| **Average Dots Per Row** | 33 to 35 dots | Highly uniform distribution along horizontal passes |
| **Matrix Type** | 2D Orthogonal Array | Straight Cartesian alignment at 90-degree intersections |

---

## 3. Dot Morphology and Optical Characteristics

- **Shape**: Circular micro-points with soft Gaussian anti-aliasing.
- **Core Footprint**: 1.0 pixel solid center at maximum opacity.
- **Subpixel Gradient**: 1-pixel radial perimeter gradient smoothly diffusing into the light gray background.
- **Visual Impact**: Subdued, rhythmic point matrix that serves as a subtle tactile backdrop rather than high-contrast polka dots.
- **Surface Regularity**: Consistent dot morphology across all coordinates with negligible distortion or skew.

---

## 4. Strict Interaction Specification (Non-Interactive Requirement)

### Absolute Rule: Zero Dynamic Interaction
- **Non-Interacting Architecture**: The dots must remain completely static and fixed in space at all times.
- **No Magnetic Cursor Reaction**: Unlike dynamic magnetic canvas components where dots attract, repel, or distort toward the pointer, these dots are strictly stationary.
- **No Cursor Displacement**: Pointer movement across the canvas produces zero coordinate translation, zero velocity impulses, and zero spring-physics deformation.
- **No State Mutation**: No hover color shifts, no size expansion, no opacity pulse, and no ripple effects upon user proximity or clicking.
- **Purpose**: Designed as a clean, peaceful architectural background canvas without visual distraction or processing overhead.
