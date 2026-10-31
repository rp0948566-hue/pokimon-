# Existing Chat Audit — GUI-P02 AI Chat Interface

## 1. Audit of Baseline Components (Post-Task 01)

| Area | Existing State | Gap / Required for Task 02 |
|---|---|---|
| **Message Store** | `Vec<ChatMessage>` in `RetinaCareApp` | Lacks `status` field (`Queued`, `Streaming`, `Completed`, `Cancelled`, `Failed`) and error detail. |
| **User Message UI** | Adaptive width bubble with text and raw badges | Lacks dedicated rich attachment card previews (thumbnails for images, tiles for PDFs). |
| **Assistant UI** | Static markdown formatting on canvas | Lacks streaming accumulation cursor, stop/cancel button, retry on failure, and truthful error cards. |
| **Action Toolbar** | Copy button only on user message | Missing copy on assistant message, retry on failed request, and active cancel control. |
| **Streaming Engine** | Synchronous message push | Needs nonblocking Rust channel receiver (`mpsc`) for streaming tokens/deltas. |
| **Scrolling Engine** | Fixed `stick_to_bottom(true)` | Needs near-bottom detection so user reading position is not interrupted when scrolling up. |
| **Composer State** | Send button only | Needs Stop button transition when response is actively streaming. |
| **Backend Boundary** | Static placeholder text | Needs formal typed `ChatService` trait and `UnavailableChatService` adapter. |

## 2. Invariant Preservation
- Task 01 Home composer layout and dimensions (740 px width, 20 px radius, `#242424` fill, `#383838` stroke) remain completely intact.
- Task 01 attachment menus and model selector remain functional.
- Zero touch to Freebuff backend code.
