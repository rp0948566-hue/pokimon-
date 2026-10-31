# Dot Size, Spacing, Visual Ratio, and Spatial Mapping

## 1. Dimensional Metrics Summary

| Property | Exact Measured Value | Nominal Display Value | Unit |
| :--- | :--- | :--- | :--- |
| **Dot Core Diameter** | 1.00 | 1.00 | Pixels (px) |
| **Effective Visual Diameter** | 1.50 to 2.00 | 1.50 | Pixels (px) |
| **Dot Radius (r)** | 0.75 to 1.00 | 0.75 | Pixels (px) |
| **Horizontal Pitch (Center-to-Center)** | 5.83 | 6.00 | Pixels (px) |
| **Vertical Pitch (Center-to-Center)** | 5.65 to 5.80 | 6.00 | Pixels (px) |
| **Horizontal Clear Gap (Edge-to-Edge)** | 4.33 | 4.50 | Pixels (px) |
| **Vertical Clear Gap (Edge-to-Edge)** | 4.15 | 4.50 | Pixels (px) |
| **Diagonal Distance (Center-to-Center)** | 8.12 to 8.36 | 8.49 | Pixels (px) |

---

## 2. Visual Point Ratio and Space Relations

### Linear Scale Relationship
- **Dot-to-Pitch Ratio (1D)**: 1.50 px diameter / 6.00 px pitch = 0.25 (1 : 4 ratio).
- **Dot-to-Gap Ratio (1D)**: 1.50 px dot / 4.50 px clear gap = 0.333 (1 : 3 ratio).
- **Interpretation**: Each dot is exactly one-third the width of the clear empty space between itself and its neighbor.

### Surface Area Occupancy (2D)
- **Dot Surface Area**: Approximated by the circle formula (pi * r^2) = 3.14159 * (0.75)^2 = 1.767 square pixels.
- **Unit Cell Surface Area**: 5.83 px * 5.80 px = 33.81 square pixels (or 36.00 square pixels at nominal 6x6 grid).
- **Dot Area Coverage**: 1.767 / 33.81 = 5.22% of total canvas space (4.91% at 6x6 pitch).
- **Clear Background Ratio**: 94.78% to 95.09% open negative space.
- **Visual Weight Outcome**: Overwhelmingly open (95% air), creating an ultra-lightweight structural guide that never clutters visual hierarchy.

---

## 3. Spatial Coordinate Mapping

### Coordinate Generation Function
- **Horizontal Axis (X)**: Position of dot at column index i is defined by:
  X(i) = X_origin + (i * Pitch_X)
  Where X_origin = 4.0 px and Pitch_X = 5.83 px (nominal 6.0 px).
- **Vertical Axis (Y)**: Position of dot at row index j is defined by:
  Y(j) = Y_origin + (j * Pitch_Y)
  Where Y_origin = 3.0 px and Pitch_Y = 5.80 px (nominal 6.0 px).

### Coordinate Table Sample (First 6 Rows and 6 Columns)

| Row \ Col | Col 0 (i=0) | Col 1 (i=1) | Col 2 (i=2) | Col 3 (i=3) | Col 4 (i=4) | Col 5 (i=5) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Row 0 (j=0)** | (4.0, 3.0) | (9.8, 3.0) | (15.7, 3.0) | (21.5, 3.0) | (27.3, 3.0) | (33.2, 3.0) |
| **Row 1 (j=1)** | (4.0, 8.8) | (9.8, 8.8) | (15.7, 8.8) | (21.5, 8.8) | (27.3, 8.8) | (33.2, 8.8) |
| **Row 2 (j=2)** | (4.0, 14.6) | (9.8, 14.6) | (15.7, 14.6) | (21.5, 14.6) | (27.3, 14.6) | (33.2, 14.6) |
| **Row 3 (j=3)** | (4.0, 20.4) | (9.8, 20.4) | (15.7, 20.4) | (21.5, 20.4) | (27.3, 20.4) | (33.2, 20.4) |
| **Row 4 (j=4)** | (4.0, 26.2) | (9.8, 26.2) | (15.7, 26.2) | (21.5, 26.2) | (27.3, 26.2) | (33.2, 26.2) |
| **Row 5 (j=5)** | (4.0, 32.0) | (9.8, 32.0) | (15.7, 32.0) | (21.5, 32.0) | (27.3, 32.0) | (33.2, 32.0) |

---

## 4. Optical Density and Visual Balance

- **Matrix Uniformity**: Strict Cartesian lattice with 90-degree orthogonal axes.
- **Repetition Frequency**: High-frequency micro-cadence (166.7 to 172.4 cycles per 1,000 pixels).
- **Subpixel Distribution**: The 5.8px frequency indicates downsampling from a baseline 6px grid or retina/standard display rendering scale.
- **Dynamic Interaction Exclusion**: Strictly fixed coordinates across all frames, ensuring absolute stability with zero motion or magnetic distortion.
