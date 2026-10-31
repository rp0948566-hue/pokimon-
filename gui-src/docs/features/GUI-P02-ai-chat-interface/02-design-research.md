# Design Research — GUI-P02 AI Chat Interface

## 1. Visual Hierarchy & Contrast
- **User Message**:
  - Distinct subtle card surface: `#222428`, stroke 1.0 px `#383C42`, corner radius 12 px.
  - Aligned to the right with adaptive width (clamped between 160 px and 78% feed width).
  - Internal header with "You", timestamp, and discrete copy action `📋`.
  - Content wraps naturally; long messages support "Show more" / "Show less" toggle.
- **Assistant Message**:
  - Sits directly on `#191919` canvas without an artificial enclosing box (following modern desktop AI UX).
  - Aligned to the left, max width matching feed width (up to 880 px).
  - Section headings rendered prominently with cyan/blue accents (`#5AAFFF`).
  - Bulleted lists with crisp spacing and accent markers (`#3B82F6`).
  - Code blocks and attachment tags rendered in monospace pill chips.
  - Action row below response: `📋 Copy`, `🔄 Retry` (on error/cancel), `⏹ Stop` (during generation).

## 2. Dynamic Status Badges
- **Queued**: Subtle grey indicator.
- **Streaming**: Active blinking cursor `▮` at the end of incoming text.
- **Completed**: Clean presentation with action bar.
- **Cancelled**: Muted amber notice `[Generation stopped by user]`.
- **Failed**: Subtle warning surface `#2B1D1D` with `#F87171` text: `AI service is not connected.` and `Retry` button.

## 3. Scrolling Ergonomics
- When user is near the bottom (< 60 px from end), new incoming tokens scroll smoothly.
- When user scrolls up (> 60 px from end) to inspect previous turns, auto-scroll pauses.
- A floating "↓ Jump to latest" pill appears when paused, allowing one-click return.
