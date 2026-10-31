# Plan: Message Stream, Multi-Modal Pipeline & Caveman Integration

Purpose: Implementation roadmap for smooth animated chat stream, multi-modal ingestion, and Caveman integration.

## Milestones
- **Cloned Caveman**: Cloned `https://github.com/JuliusBrussee/caveman.git` into `./caveman/` for token reduction and compression analysis.
- **Architectural Specifications**: Documented complete multi-modal ingestion pipelines (text, high-res fundus images, multi-page PDFs) in `docs/architecture.md`.
- **Smooth Bottom-Docking Animation**:
  - Typing bar starts centered on empty Home page.
  - On prompt submission, smoothly docks to the bottom edge using `cubic_out` animation interpolation.
- **Formatted User Message Bubble (Image 1 Reference)**:
  - Card container `#222428`, `#383C42` stroke, 12px radius.
  - Large message handling: expandable `"Show more ▾"` / `"Show less ▴"` toggle for long prompts (>5 lines / >200 chars).
  - Timestamp and copy action controls.
- **Unboxed Assistant Response Stream (Image 2 Reference)**:
  - Directly rendered on `#191919` canvas (no card container).
  - Rich typography: section headings, bold bullet points (`•`), and inline code badges (`.bat`, `settings.json`).
