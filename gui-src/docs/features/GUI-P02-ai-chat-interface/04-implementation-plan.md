# Implementation Plan — GUI-P02 AI Chat Interface

## 1. Plan Structure

### Step 1: Domain Extension (`rust/src/domain.rs`)
- Add `ChatMessageStatus`: `Queued`, `Streaming`, `Completed`, `Cancelled`, `Failed`.
- Add `status` and `error_detail` to `ChatMessage`.
- Define `ChatEvent`: `Started`, `Delta`, `Completed`, `Failed`, `Cancelled`.
- Add helper methods on `ChatMessage` for appending deltas, completing, cancelling, and failing.

### Step 2: Typed Chat Service & Contract (`rust/src/engine/chat.rs` or `rust/src/domain.rs`)
- Define `ChatRequest`, `RequestId`, `ChatServiceError`.
- Define `ChatService` trait: `submit(&self, req) -> Result<RequestId, ChatServiceError>` and `cancel(&self, id)`.
- Implement `UnavailableChatService` providing truthful "AI service is not connected."
- Implement a testable `MockStreamingChatService` for automated test suites.

### Step 3: Assistant Message UI & Toolbar (`rust/src/main.rs`)
- Add Copy, Retry, Stop actions to assistant messages.
- Render streaming cursor `▮` when status is `Streaming`.
- Render truthful failure state with Retry button when status is `Failed`.
- Render cancellation note when status is `Cancelled`.

### Step 4: User Message Rich Attachments (`rust/src/main.rs`)
- Render image attachment cards with thumbnail previews.
- Render PDF attachment cards with page counts and metadata.
- Clean plain-text copy action.

### Step 5: Streaming Event Receiver in App Update Loop (`rust/src/main.rs`)
- Add `mpsc::Receiver<ChatEvent>` and active `Option<String>` request ID to `RetinaCareApp`.
- Nonblocking `try_recv()` loop in `update()` to consume deltas without frame latency.

### Step 6: Scroll Mechanics & Jump-to-Latest Control (`rust/src/main.rs`)
- Track scroll offset and content height.
- Show "↓ Jump to latest" button when user is scrolled up and generation is active.

### Step 7: Tests & Verification
- Comprehensive unit tests covering all message transitions, streaming events, copy, retry, and cancellation.
- Clippy check, release build, and desktop screenshot capture.
