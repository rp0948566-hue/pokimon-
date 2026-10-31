# Background and Dot Colour Analysis

## 1. Overview
Comprehensive colorimetric inspection of the provided canvas image, analyzing the dominant background plane, color distribution, dot color values, and relative contrast ratios.

---

## 2. Background Color Specification

| Metric | Primary Value | Variance Range | Notes |
| :--- | :--- | :--- | :--- |
| **Hexadecimal** | #D8D8D8 | #D6D6D6 to #DADADA | Very light neutral gray / warm silver |
| **RGB** | 216, 216, 216 | 214 to 218 across channels | Completely neutral gray (R = G = B) |
| **HSL** | 0 deg, 0%, 84.7% | Lightness 83.9% to 85.5% | Pure achromatic lightness |
| **CMYK** | 0%, 0%, 0%, 15.3% | K: 14.5% to 16.1% | Zero tint, black-only contribution |
| **Relative Luminance** | 0.6878 | 0.6720 to 0.7010 | High-luminance reflective surface |
| **Channel Uniformity** | 100% Monochrome | Zero color divergence | Red, Green, and Blue channels are identical |

---

## 3. Background Pixel Distribution

| Hex Code | RGB Values | Pixel Count | Surface Percentage | Role |
| :--- | :--- | :--- | :--- | :--- |
| **#D8D8D8** | (216, 216, 216) | 5,328 | 27.21% | Modal background base |
| **#D7D7D7** | (215, 215, 215) | 3,734 | 19.07% | Ambient subtle shading |
| **#D9D9D9** | (217, 217, 217) | 3,515 | 17.95% | Ambient subtle highlight |
| **#D6D6D6** | (214, 214, 214) | 1,760 | 8.99% | Subtle lower boundary |
| **#DADADA** | (218, 218, 218) | 1,673 | 8.54% | Subtle upper boundary |
| **#D5D5D5 to #DBDBDB** | (213 to 219) | 3,574 | 18.24% | Gradual boundary transitions |

---

## 4. Dot Color Specification

| Dot Layer | Hex Code | RGB Values | Relative Luminance | Visual Function |
| :--- | :--- | :--- | :--- | :--- |
| **Absolute Core Minimum** | #BCBCBC | (188, 188, 188) | 0.5055 | Deepest focal center of distinct dots |
| **Primary Dot Center** | #C4C4C4 to #C7C7C7 | (196 to 199) | 0.5539 to 0.5749 | Dominant core pixel color across grid |
| **Transitional Antialiasing** | #D0D0D0 to #D5D5D5 | (208 to 213) | 0.6322 to 0.6668 | Soft radial falloff into background |

---

## 5. Contrast and Luminance Evaluation

- **Luminance Difference**: 0.1129 between mean background (0.6878) and mean dot center (0.5749).
- **Contrast Ratio**: 1.18 : 1 (relative to background).
- **Visual Weight**: Extremely low-contrast micro-texture designed to avoid visual fatigue, providing spatial structure without overpowering foreground elements.
- **Equivalent Tone**: Matches approximately 8.5% to 10% black opacity placed directly on a #D8D8D8 base.
