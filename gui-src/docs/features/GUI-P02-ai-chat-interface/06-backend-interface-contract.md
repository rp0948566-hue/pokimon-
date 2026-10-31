# Backend Interface Contract — GUI-P02 AI Chat Interface

## 1. Frontend-Facing Contract for Codex / Freebuff

```rust
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub request_id: String,
    pub session_id: String,
    pub prompt: String,
    pub attachments: Vec<crate::domain::LocalAttachment>,
    pub model_identifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Started { request_id: String, message_id: String },
    Delta { request_id: String, text: String },
    Completed { request_id: String },
    Failed { request_id: String, error: String },
    Cancelled { request_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatServiceError {
    NotConnected,
    Busy,
    InvalidPayload(String),
}

pub trait ChatService: Send + Sync {
    fn submit(
        &self,
        request: ChatRequest,
        event_sender: Sender<ChatEvent>,
    ) -> Result<(), ChatServiceError>;

    fn cancel(&self, request_id: &str) -> Result<(), ChatServiceError>;
}
```

## 2. Truthful Unavailable Adapter
When Codex's backend is not connected:
```rust
pub struct UnavailableChatService;

impl ChatService for UnavailableChatService {
    fn submit(&self, req: ChatRequest, sender: Sender<ChatEvent>) -> Result<(), ChatServiceError> {
        let _ = sender.send(ChatEvent::Failed {
            request_id: req.request_id,
            error: "AI service is not connected. Connect Freebuff backend to begin clinical consultation.".to_string(),
        });
        Ok(())
    }

    fn cancel(&self, _request_id: &str) -> Result<(), ChatServiceError> {
        Ok(())
    }
}
```
This guarantees no simulated chatbot and zero fabricated diagnoses while testing the real event flow.
