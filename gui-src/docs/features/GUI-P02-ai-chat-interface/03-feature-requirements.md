# Feature Requirements — GUI-P02 AI Chat Interface

## 1. Functional Requirements

### FR-01: Message Submission Flow
- Reject empty prompt if no attachments exist.
- Accept attachment-only messages (e.g. fundus image or PDF report without text).
- Do not clear the composer draft before local message insertion succeeds.
- Prevent duplicate concurrent submissions while generation is in flight.

### FR-02: User Message Presentation
- Right-aligned adaptive-width container.
- Embedded image attachments render with compact thumbnail, display name, and size.
- Embedded PDF attachments render with document icon, page count, and size.
- Copy button copies plain text prompt to system clipboard.

### FR-03: Assistant Message Presentation
- Left-aligned clean typography directly on canvas.
- Real-time status visualization: Queued, Streaming, Completed, Cancelled, Failed.
- Markdown rendering for headings (`###`), lists (`•`), inline code, and sub-items.
- Action row: Copy, Retry (failed/cancelled), Stop (in-flight).

### FR-04: Nonblocking Streaming Architecture
- Nonblocking Rust channel receiver (`mpsc::channel`) polled per egui frame.
- Safely appends incremental text deltas without blocking the GUI.
- Out-of-order or mismatched request IDs are ignored.

### FR-05: Scroll Behavior & Viewport
- Auto-scroll to bottom only when user is near bottom.
- Pause auto-scroll when user manually scrolls up.
- Jump-to-bottom control when scrolled up and new text arrives.

### FR-06: Backend Contract Boundary
- Define `ChatService` trait for future Codex backend implementation.
- Provide `UnavailableChatService` adapter reporting truthful not-connected status.
- Zero fake diagnostic inference or fabricated clinical data.
