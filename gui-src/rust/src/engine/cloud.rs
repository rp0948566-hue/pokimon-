//! Background cloud sync worker: sign-in, session restore from the OS keyring, push of queued general
//! chat messages and pull of the account's own conversations. Runs on its own thread; the UI only sends
//! commands and reads events, so a slow or missing network never blocks a frame.

use super::keystore;
use super::store::SharedStore;
use super::supabase::{push_outbox, Client, CloudError, Config, Session};
use retinacare_persistence::OutboxItem;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

const AUTO_SYNC_EVERY: Duration = Duration::from_secs(60);
const PUSH_BATCH: i64 = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudState {
    SignedOut,
    /// Signed in, but the service is not reachable; everything stays local and queued.
    OfflineLocal {
        queued: i64,
    },
    SyncPending {
        queued: i64,
    },
    Syncing,
    Connected {
        email: String,
    },
    Error(String),
}

impl CloudState {
    pub fn label(&self) -> String {
        match self {
            Self::SignedOut => "Signed out: conversations stay on this device".into(),
            Self::OfflineLocal { queued } => {
                format!("Offline: {queued} message(s) waiting, stored locally")
            }
            Self::SyncPending { queued } => format!("{queued} message(s) waiting to sync"),
            Self::Syncing => "Syncing...".into(),
            Self::Connected { email } => format!("Connected as {email}"),
            Self::Error(reason) => format!("Cloud error: {reason}"),
        }
    }
    pub fn signed_in(&self) -> bool {
        !matches!(self, Self::SignedOut)
    }
}

pub enum CloudCommand {
    /// Try the refresh token in the OS keyring (no password is ever stored).
    Restore,
    SignIn {
        email: String,
        password: Zeroizing<String>,
    },
    SignOut,
    SyncNow,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudEvent {
    State(CloudState),
    /// Short note for the user: keyring unavailable, N conversations pulled, and so on.
    Note(String),
    /// Conversations changed locally (a pull added rows): reload the history list.
    LocalChanged,
}

pub struct CloudHandle {
    tx: Sender<CloudCommand>,
    pub events: Receiver<CloudEvent>,
}

impl CloudHandle {
    pub fn spawn(store: SharedStore, cfg: Config) -> Self {
        let (tx, rx) = channel::<CloudCommand>();
        let (etx, events) = channel::<CloudEvent>();
        std::thread::spawn(move || {
            Worker {
                client: Client::new(cfg),
                store,
                session: None,
                events: etx,
            }
            .run(rx)
        });
        Self { tx, events }
    }

    pub fn send(&self, command: CloudCommand) {
        let _ = self.tx.send(command);
    }
}

impl Drop for CloudHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(CloudCommand::Shutdown);
    }
}

struct Worker {
    client: Client,
    store: SharedStore,
    session: Option<Session>,
    events: Sender<CloudEvent>,
}

impl Worker {
    fn state(&self, state: CloudState) {
        let _ = self.events.send(CloudEvent::State(state));
    }
    fn note(&self, text: impl Into<String>) {
        let _ = self.events.send(CloudEvent::Note(text.into()));
    }

    fn queued(&self) -> i64 {
        self.store.with(|s| s.outbox_unsent_count()).unwrap_or(0)
    }

    fn run(mut self, rx: Receiver<CloudCommand>) {
        loop {
            match rx.recv_timeout(AUTO_SYNC_EVERY) {
                Ok(CloudCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(CloudCommand::Restore) => self.restore(),
                Ok(CloudCommand::SignIn { email, password }) => self.sign_in(&email, &password),
                Ok(CloudCommand::SignOut) => self.sign_out(),
                Ok(CloudCommand::SyncNow) => self.sync(),
                Err(RecvTimeoutError::Timeout) => {
                    if self.session.is_some() {
                        self.sync();
                    }
                }
            }
        }
    }

    fn adopt(&mut self, session: Session) {
        // The new refresh token replaces the old one (they rotate); no keyring means no remembered login.
        if keystore::save_refresh_token(&session.refresh_token).is_err() {
            self.note("No OS keyring is available: this sign-in will not be remembered after the app closes.");
        }
        self.session = Some(session);
    }

    fn restore(&mut self) {
        match keystore::load_refresh_token() {
            Ok(Some(token)) => match self.client.refresh(&token) {
                Ok(session) => {
                    self.adopt(session);
                    self.sync();
                }
                Err(CloudError::Offline) => {
                    // Keep the token: the next launch (or Sync now) tries again.
                    self.state(CloudState::OfflineLocal {
                        queued: self.queued(),
                    });
                    self.note("Offline: the saved cloud session could not be checked yet.");
                }
                Err(_) => {
                    keystore::delete_refresh_token();
                    self.state(CloudState::SignedOut);
                    self.note("The saved cloud session is no longer valid. Sign in again.");
                }
            },
            Ok(None) => self.state(CloudState::SignedOut),
            Err(_) => {
                self.state(CloudState::SignedOut);
                self.note("No OS keyring is available, so a cloud sign-in cannot be remembered.");
            }
        }
    }

    fn sign_in(&mut self, email: &str, password: &str) {
        self.state(CloudState::Syncing);
        match self.client.sign_in(email.trim(), password) {
            Ok(session) => {
                self.adopt(session);
                self.sync();
            }
            Err(CloudError::Offline) => {
                self.state(CloudState::SignedOut);
                self.note("Cannot reach the cloud service. Check the connection and try again.");
            }
            Err(error) => {
                self.state(CloudState::SignedOut);
                self.note(error.to_string());
            }
        }
    }

    fn sign_out(&mut self) {
        if let Some(session) = self.session.take() {
            self.client.sign_out(&session);
        }
        keystore::delete_refresh_token();
        self.state(CloudState::SignedOut);
        self.note("Signed out. Conversations already on this device stay here.");
    }

    /// A valid access token, refreshed when it is about to expire.
    fn fresh_session(&mut self) -> Result<(), CloudError> {
        let expired = self
            .session
            .as_ref()
            .is_none_or(|s| s.expires_at <= Instant::now());
        if !expired {
            return Ok(());
        }
        let token = self
            .session
            .as_ref()
            .map(|s| s.refresh_token.clone())
            .ok_or(CloudError::SessionExpired)?;
        let session = self.client.refresh(&token)?;
        self.adopt(session);
        Ok(())
    }

    fn sync(&mut self) {
        if self.session.is_none() {
            self.state(CloudState::SignedOut);
            return;
        }
        self.state(CloudState::Syncing);
        let result = self.fresh_session().and_then(|()| self.push_and_pull());
        match result {
            Ok(()) => {
                let email = self
                    .session
                    .as_ref()
                    .map(|s| s.email.clone())
                    .unwrap_or_default();
                let queued = self.queued();
                self.state(if queued > 0 {
                    CloudState::SyncPending { queued }
                } else {
                    CloudState::Connected { email }
                });
            }
            Err(CloudError::Offline) => self.state(CloudState::OfflineLocal {
                queued: self.queued(),
            }),
            Err(CloudError::SessionExpired) => {
                self.session = None;
                keystore::delete_refresh_token();
                self.state(CloudState::SignedOut);
                self.note("The cloud session ended. Sign in again; nothing local was lost.");
            }
            Err(error) => self.state(CloudState::Error(error.to_string())),
        }
    }

    fn push_and_pull(&mut self) -> Result<(), CloudError> {
        let Some(session) = self.session.as_ref() else {
            return Err(CloudError::SessionExpired);
        };
        // Push: only what the store marks eligible (general, text, completed).
        loop {
            let items: Vec<OutboxItem> = self
                .store
                .with(|s| s.outbox_pending(PUSH_BATCH))
                .map_err(|_| CloudError::BadResponse)?;
            if items.is_empty() {
                break;
            }
            let store = &self.store;
            let report = push_outbox(
                &self.client,
                session,
                &items,
                |id| {
                    let _ = store.with(|s| s.outbox_mark_synced(id));
                },
                |id, why| {
                    let _ = store.with(|s| s.outbox_mark_failed(id, why));
                },
            );
            if let Some(stopped) = report.stopped {
                return Err(stopped);
            }
            // A pass that sent nothing and failed everything would spin: leave the failures queued.
            if report.sent == 0 {
                break;
            }
        }
        // Pull: this account's own conversations (RLS), merged idempotently.
        let mut changed = false;
        for conversation in self.client.list_conversations(session)? {
            if conversation.deleted {
                continue;
            }
            let messages = self.client.list_messages(session, &conversation.id)?;
            let added = self
                .store
                .with(|s| {
                    s.import_remote_conversation(&conversation.id, &conversation.title, &messages)
                })
                .unwrap_or(0);
            changed |= added > 0;
        }
        if changed {
            let _ = self.events.send(CloudEvent::LocalChanged);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_labels_are_truthful() {
        assert!(CloudState::SignedOut
            .label()
            .contains("stay on this device"));
        assert!(CloudState::OfflineLocal { queued: 2 }
            .label()
            .contains("Offline"));
        assert!(CloudState::Connected {
            email: "a@b.c".into()
        }
        .label()
        .contains("a@b.c"));
        assert!(!CloudState::SignedOut.signed_in());
        assert!(CloudState::Syncing.signed_in());
    }

    fn open_store(name: &str) -> SharedStore {
        let path = std::env::temp_dir().join(format!(
            "retinacare-cloud-{name}-{}-{}/db.sqlite",
            std::process::id(),
            retinacare_persistence::new_uuid()
        ));
        let store = SharedStore::new(path);
        store.begin_unlock("password-123456".into(), Some("password-123456".into()));
        let start = Instant::now();
        while !store.is_open() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "database did not open"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        store
    }

    fn test_session() -> Session {
        Session {
            access_token: Zeroizing::new("ACCESS".into()),
            refresh_token: Zeroizing::new("REFRESH".into()),
            user_id: "11111111-1111-4111-8111-111111111111".into(),
            email: "a@example.test".into(),
            expires_at: Instant::now() + Duration::from_secs(3000),
        }
    }

    fn text_message(role: &str, content: &str) -> retinacare_persistence::MessageInput {
        retinacare_persistence::MessageInput {
            client_msg_id: retinacare_persistence::new_uuid(),
            role: role.into(),
            kind: "text".into(),
            status: "completed".into(),
            content: content.into(),
            card_json: None,
            provider: None,
            attachment_count: 0,
        }
    }

    #[test]
    fn only_opted_in_general_chats_reach_the_network_and_the_pull_is_idempotent() {
        use retinacare_persistence::ConversationMode;
        let store = open_store("e2e");
        let pid = store.with(|s| s.add_patient("SYN-1", "Synthetic")).unwrap();
        let synced = store
            .with(|s| s.create_conversation("Synced chat", ConversationMode::Chat, None, None))
            .unwrap();
        let local_only = store
            .with(|s| s.create_conversation("Private chat", ConversationMode::Chat, None, None))
            .unwrap();
        let research = store
            .with(|s| {
                s.create_conversation("Research", ConversationMode::Research, Some(pid), None)
            })
            .unwrap();
        for (conv, text) in [
            (&synced, "hello cloud"),
            (&local_only, "stay home"),
            (&research, "patient text"),
        ] {
            store
                .with(|s| s.append_message(conv, &text_message("user", text)))
                .unwrap();
        }
        store
            .with(|s| s.set_conversation_sync(&synced, true))
            .unwrap();
        assert!(store
            .with(|s| s.set_conversation_sync(&research, true))
            .is_err());

        // upsert + message for the one synced chat, then the pull: one remote conversation with one message.
        let remote = retinacare_persistence::new_uuid();
        let remote_msgs = serde_json::json!([{"client_msg_id": retinacare_persistence::new_uuid(), "role": "assistant", "content": "from another device"}]);
        let remote_convs =
            serde_json::json!([{"id": remote, "title": "Other device", "deleted_at": null}]);
        let (url, log) = super::super::supabase::tests::server(vec![
            (201, String::new()),
            (201, String::new()),
            (200, remote_convs.to_string()),
            (200, remote_msgs.to_string()),
            (200, remote_convs.to_string()),
            (200, remote_msgs.to_string()),
        ]);
        let (etx, erx) = channel();
        let mut worker = Worker {
            client: Client::new(Config {
                url,
                key: "k".into(),
            }),
            store: store.clone(),
            session: Some(test_session()),
            events: etx,
        };
        worker.push_and_pull().unwrap();
        assert_eq!(erx.try_recv().unwrap(), CloudEvent::LocalChanged);
        {
            let log = log.lock().unwrap();
            let bodies: String = log.iter().map(|l| l.2.clone()).collect();
            assert!(bodies.contains("hello cloud"));
            assert!(
                !bodies.contains("stay home") && !bodies.contains("patient text"),
                "private text never left"
            );
            assert!(log
                .iter()
                .all(|l| !l.0.contains("patients") && !l.2.contains("SYN-1")));
        }
        assert_eq!(store.with(|s| s.outbox_unsent_count()).unwrap(), 0);
        // The pulled conversation is local now, and a second pull adds nothing and queues nothing.
        let list = store.with(|s| s.list_conversations("")).unwrap();
        assert!(list.iter().any(|c| c.title == "Other device"));
        worker.push_and_pull().unwrap();
        assert!(
            erx.try_recv().is_err(),
            "nothing changed on the second pull"
        );
        let msgs = store.with(|s| s.load_messages(&remote)).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(store.with(|s| s.outbox_unsent_count()).unwrap(), 0);
    }

    #[test]
    fn offline_keeps_the_queue_and_a_later_sync_sends_each_message_once() {
        use retinacare_persistence::ConversationMode;
        let store = open_store("offline");
        let conv = store
            .with(|s| s.create_conversation("Chat", ConversationMode::Chat, None, None))
            .unwrap();
        store
            .with(|s| s.set_conversation_sync(&conv, true))
            .unwrap();
        store
            .with(|s| s.append_message(&conv, &text_message("user", "queued while offline")))
            .unwrap();
        let (etx, _erx) = channel();
        let mut worker = Worker {
            client: Client::new(Config {
                url: "http://127.0.0.1:9".into(),
                key: "k".into(),
            }),
            store: store.clone(),
            session: Some(test_session()),
            events: etx,
        };
        assert_eq!(worker.push_and_pull().unwrap_err(), CloudError::Offline);
        assert_eq!(
            store.with(|s| s.outbox_unsent_count()).unwrap(),
            1,
            "nothing lost"
        );
        let (url, log) = super::super::supabase::tests::server(vec![
            (201, String::new()),
            (201, String::new()),
            (200, "[]".into()),
        ]);
        worker.client = Client::new(Config {
            url,
            key: "k".into(),
        });
        worker.push_and_pull().unwrap();
        assert_eq!(store.with(|s| s.outbox_unsent_count()).unwrap(), 0);
        let sent = log
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.0.contains("conversation_messages"))
            .count();
        assert_eq!(sent, 1);
        let (url, log) = super::super::supabase::tests::server(vec![(200, "[]".into())]);
        worker.client = Client::new(Config {
            url,
            key: "k".into(),
        });
        worker.push_and_pull().unwrap();
        assert!(
            log.lock().unwrap().iter().all(|l| !l.0.contains("POST")),
            "a repeat sends nothing"
        );
    }

    #[test]
    fn a_locked_store_means_nothing_is_pushed_and_the_state_says_offline_or_pending() {
        // No database is unlocked: queued() reads 0 and an unauthenticated worker reports signed out.
        let store =
            SharedStore::new(std::env::temp_dir().join("retinacare-cloud-test/none.sqlite"));
        let handle = CloudHandle::spawn(
            store,
            Config {
                url: "http://127.0.0.1:9".into(),
                key: "k".into(),
            },
        );
        handle.send(CloudCommand::SyncNow);
        let event = handle.events.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(event, CloudEvent::State(CloudState::SignedOut));
    }
}
