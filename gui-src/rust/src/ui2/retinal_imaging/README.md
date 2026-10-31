# Retinal Imaging page

One page, one folder. Everything page-specific lives here; the rest of the app only registers it.

## What it shows
- Left: the open examination's original fundus (larger). Right: a same-examination preview chosen from the strip (smaller), or an empty state.
- Four preview cards: Original Fundus, Vessel Mask, Lesion Overlay, Explainability. A card opens only when the matching saved artifact exists. Clicking opens it large and centered; `Back` exists only in that state.
- Right rail, five cards: Image View, Image Enhancement, Analysis Tools, Image Management, Display Settings.

## Contract
- Read-only. No database, no inference, no file writes, no network.
- `RetinalImagingContext` borrows the app's current `RetinalExaminationView` and original texture. The page keeps no patient data.
- `RetinalImagingState` holds view preferences only, bound to `patient|exam|eye`. Any change of that key resets it (`bind`).
- `RetinalImagingAction` is the only way out: `OpenReview`, `OpenReport`, `None`. The app decides what to do.
- Overlays use the existing `render_*_overlay` functions on the same fitted rectangle as the photo, so they stay aligned.
- Brightness is a draw-time tint. Stored pixels never change.
- FA, ICG, OCT and Red-Free are listed but disabled: the app stores no such data. The preview cards are real outputs only.

## Controls (live vs unavailable)
Live (display-only unless stated): Brightness, Contrast, Saturation (cached adjusted texture; `adjust.rs`), simulated Red-Free (green channel, labelled as such), wheel zoom + drag pan, Reset, Annotations (temporary markers, add/remove last/clear, never stored), Measurement (pixel distance only, never mm), Save Image (PNG via file dialog; file name says `original` or `adjusted_view`), Vessel Analysis, Lesion Detection, DR Grading (saved headline), Grid, Export (opens the report page), tabs Analysis and Report, preview cards, Back.
Unavailable, with a hover reason: FA, ICG, OCT (no data), Add to Study (no study model), Color Map (overlay palettes are fixed by the existing analysis pages), AI Analysis (status only), Comparison tab.
Contrast/Saturation/Red-Free/Save need the source file path; without it they stay disabled.

## App integration (the only edits outside this folder)
- `ui2/mod.rs`: `pub mod retinal_imaging;`
- `ui2/shell.rs`, `ui2/icon_paths.rs`: `Icon::Imaging` glyph.
- `main.rs`: `AppPage::RetinalImaging`, one `PAGES` and `nav` entry, one state field, one dispatch arm, one legacy-sidebar no-op arm, one `RETINACARE_PAGE=imaging` dev value.

## Rollback
Delete the `PAGES` entry and the matching `nav` entry in `chrome_v2` (main.rs). The page becomes unreachable and nothing else changes. To remove it fully, also delete this folder and the lines listed above.

Test: `cargo test retinal_imaging` (from `rust/`).
