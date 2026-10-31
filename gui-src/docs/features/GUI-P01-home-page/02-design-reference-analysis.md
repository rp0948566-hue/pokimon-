# Design Reference Analysis — GUI-P01 Home Page

## Design Language: Minimalist Clinical Dark Workspace

### 1. Spatial Proportions & Alignment
- **Workspace Canvas**: Dark background (`#191919`) occupying all remaining screen space to the right of the 260px fixed sidebar.
- **Composer Proportions**:
  - Desktop Width: ~680–770 logical pixels (calibrated to 740 px on standard 1080p and scaled responsively down to 440 px on compact screens).
  - Minimum Height: 115–125 logical pixels (expanding smoothly when attachments or multiline text are added).
  - Corner Radius: 20 pixels (`CornerRadius::same(20)`).
  - Vertical Positioning: Visually balanced in the center of the available workspace height when in empty state.
  - Horizontal Positioning: Centered strictly within `main_rect` (between sidebar and right window border).

### 2. Color Palette & Typography Tokens
- **Window Shell**: `#171717` (Deep neutral dark background)
- **Workspace Canvas**: `#191919` (Canvas surface)
- **Composer Fill**: `#242424` (Subtly raised tactile dark charcoal)
- **Raised Controls & Buttons**: `#2D2D2D`
- **Neutral Borders**: `#383838` (Subtle 1px perimeter stroke)
- **Primary Text**: `#F1F1F1` (High-contrast, crisp readable typography)
- **Secondary Text**: `#A3A3A3` (Muted action labels and descriptions)
- **Muted Text / Hints**: `#787878` (Placeholder and inactive state text)

### 3. Composer Internal Ergonomics
- **Top Row**: Clean multiline text editor with 15.5px proportional font, `Ask anything, or task an agent...` placeholder.
- **Bottom Left**: Minimal `+` attachment button (subtle rounded rect, hover highlighting).
- **Bottom Right**:
  - Compact model selector button showing truthful status (e.g. `HierarchiRetina (Awaiting Backend)` or `Offline Mode`).
  - Circular Send button (36px diameter) with `↑` icon; enabled only when text or attachments are present.

### 4. Supporting Utility Row
- Centered directly below the composer with a 14px vertical gap.
- Width matched to the composer width for unified visual harmony.
- Includes:
  - `[📁 Choose Project]` button
  - `[🧩 Plugins]` button
- Clean minimal pills with 12px text, subtle borders (`#383838`), and dark fills (`#2D2D2D`).
