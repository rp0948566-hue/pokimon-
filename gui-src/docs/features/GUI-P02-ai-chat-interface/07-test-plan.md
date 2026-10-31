# Test Plan — GUI-P02 AI Chat Interface

## 1. Unit & Integration Test Specifications

| Test Case | Description | Target Component |
|---|---|---|
| `test_chat_message_creation_stable_id` | Verifies stable message ID format and non-reliance on array index | `domain::ChatMessage` |
| `test_chat_message_status_transitions` | Tests Queued -> Streaming -> Completed | `domain::ChatMessage` |
| `test_streaming_delta_accumulation` | Verifies sequential token concatenation | `domain::ChatMessage` |
| `test_streaming_late_event_ignored` | Verifies events for mismatched request ID are discarded | `domain::ChatMessage` |
| `test_chat_cancellation_preserves_text`| Verifies partial text preservation on stop | `domain::ChatMessage` |
| `test_chat_retry_resets_error_state` | Verifies retry resets message status to Queued/Streaming | `domain::ChatMessage` |
| `test_empty_message_validation` | Rejects empty text when no attachments exist | `RetinaCareApp` |
| `test_attachment_only_submission` | Allows submission with image/PDF even if prompt text is empty | `RetinaCareApp` |
| `test_unavailable_chat_service` | Confirms truthful failure event without fabricating answers | `UnavailableChatService` |
| `test_session_isolation_on_switch` | Verifies messages don't leak between different patient sessions | `RetinaCareApp` |
| `test_copy_message_plain_text` | Confirms clipboard text contains only prompt/answer, no raw paths | `RetinaCareApp` |

## 2. Regression Tests
- All 40 existing tests from Task 01 must pass without modification.
- Clippy must pass with `-D warnings`.
- Release build must compile without error.
