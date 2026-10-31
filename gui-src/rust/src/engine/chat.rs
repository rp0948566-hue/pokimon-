//! Typed Chat Service contract and frontend boundary for RetinaCare
use crate::domain::{ChatEvent, ChatRequest};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatServiceError {
    NotConnected,
    Busy,
    InvalidPayload(String),
}

impl std::fmt::Display for ChatServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "AI service is not connected."),
            Self::Busy => write!(
                f,
                "AI service is currently busy processing another request."
            ),
            Self::InvalidPayload(msg) => write!(f, "Invalid payload: {}", msg),
        }
    }
}

impl std::error::Error for ChatServiceError {}

/// Frontend-facing contract for background AI service integration
pub trait ChatService: Send + Sync {
    /// Dispatches a chat request to the service
    fn submit(
        &self,
        request: ChatRequest,
        event_sender: Sender<ChatEvent>,
    ) -> Result<(), ChatServiceError>;

    /// Requests cancellation of an in-flight request
    fn cancel(&self, request_id: &str) -> Result<(), ChatServiceError>;
}

/// Truthful adapter when no real AI backend is connected.
/// Never synthesizes fake diagnoses or fabricated AI responses.
#[derive(Debug, Default, Clone)]
pub struct UnavailableChatService;

impl ChatService for UnavailableChatService {
    fn submit(
        &self,
        request: ChatRequest,
        event_sender: Sender<ChatEvent>,
    ) -> Result<(), ChatServiceError> {
        let req_id = request.request_id;
        let _ = event_sender.send(ChatEvent::Started {
            request_id: req_id.clone(),
            message_id: format!("asst_{}", req_id),
        });
        let _ = event_sender.send(ChatEvent::Failed {
            request_id: req_id,
            error: "AI service is not connected. Connect Freebuff backend to begin clinical consultation.".to_string(),
        });
        Ok(())
    }

    fn cancel(&self, _request_id: &str) -> Result<(), ChatServiceError> {
        Ok(())
    }
}

/// Deterministic mock service used strictly for automated tests of streaming events
#[derive(Debug, Default, Clone)]
pub struct MockStreamingChatService {
    pub tokens: Vec<String>,
}

impl MockStreamingChatService {
    pub fn new(tokens: Vec<&str>) -> Self {
        Self {
            tokens: tokens.into_iter().map(String::from).collect(),
        }
    }
}

impl ChatService for MockStreamingChatService {
    fn submit(
        &self,
        request: ChatRequest,
        event_sender: Sender<ChatEvent>,
    ) -> Result<(), ChatServiceError> {
        let req_id = request.request_id;
        let _ = event_sender.send(ChatEvent::Started {
            request_id: req_id.clone(),
            message_id: format!("asst_{}", req_id),
        });
        for token in &self.tokens {
            let _ = event_sender.send(ChatEvent::Delta {
                request_id: req_id.clone(),
                text: token.clone(),
            });
        }
        let _ = event_sender.send(ChatEvent::Completed { request_id: req_id });
        Ok(())
    }

    fn cancel(&self, _request_id: &str) -> Result<(), ChatServiceError> {
        // Mock supports immediate cancellation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_unavailable_chat_service() {
        let service = UnavailableChatService;
        let (tx, rx) = channel();
        let req = ChatRequest {
            request_id: "req_test_1".to_string(),
            session_id: "sess_1".to_string(),
            prompt: "Evaluate OD scan".to_string(),
            attachments: vec![],
            model_identifier: "HierarchiRetina".to_string(),
            history: Vec::new(),
            results_context: None,
        };

        let res = service.submit(req, tx);
        assert!(res.is_ok());

        let ev1 = rx.recv().expect("Event 1");
        assert_eq!(
            ev1,
            ChatEvent::Started {
                request_id: "req_test_1".to_string(),
                message_id: "asst_req_test_1".to_string(),
            }
        );

        let ev2 = rx.recv().expect("Event 2");
        match ev2 {
            ChatEvent::Failed { request_id, error } => {
                assert_eq!(request_id, "req_test_1");
                assert!(error.contains("AI service is not connected"));
            }
            _ => panic!("Expected failed event"),
        }
    }

    #[test]
    fn test_mock_streaming_chat_service() {
        let service = MockStreamingChatService::new(vec!["Hello ", "world!"]);
        let (tx, rx) = channel();
        let req = ChatRequest {
            request_id: "req_test_2".to_string(),
            session_id: "sess_1".to_string(),
            prompt: "Test query".to_string(),
            attachments: vec![],
            model_identifier: "Mock".to_string(),
            history: Vec::new(),
            results_context: None,
        };

        service.submit(req, tx).expect("Submit succeeds");
        let ev1 = rx.recv().expect("Event 1");
        assert_eq!(
            ev1,
            ChatEvent::Started {
                request_id: "req_test_2".to_string(),
                message_id: "asst_req_test_2".to_string(),
            }
        );

        let ev2 = rx.recv().expect("Event 2");
        assert_eq!(
            ev2,
            ChatEvent::Delta {
                request_id: "req_test_2".to_string(),
                text: "Hello ".to_string(),
            }
        );

        let ev3 = rx.recv().expect("Event 3");
        assert_eq!(
            ev3,
            ChatEvent::Delta {
                request_id: "req_test_2".to_string(),
                text: "world!".to_string(),
            }
        );

        let ev4 = rx.recv().expect("Event 4");
        assert_eq!(
            ev4,
            ChatEvent::Completed {
                request_id: "req_test_2".to_string(),
            }
        );
    }
}
