//! Optional cloud chat sync against the RetinaCare Supabase project (research prototype).
//!
//! Only the project URL and the *publishable* key are used here; both are public by design. Row Level
//! Security on the server scopes every row to the signed-in user, so this client sends the user's own
//! access token and nothing privileged. Tokens live in memory (`Zeroizing`) and in the OS keyring (the
//! refresh token); they are never logged, never written to a file and never part of an error text.
//!
//! What may be sent is decided before this module: `outbox_pending` returns only text messages of
//! general (non-patient, attachment-free) chat conversations. Responses are parsed as data and never
//! treated as instructions.

use retinacare_persistence::{new_uuid, MessageInput, OutboxItem};
use serde::Deserialize;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

pub const PROJECT_REF: &str = "ebflochdtajcflmlhgqa";
pub const DEFAULT_URL: &str = "https://ebflochdtajcflmlhgqa.supabase.co";
/// Publishable (anon-role) key. Public by design; safe to ship. The service-role key must never appear here.
pub const PUBLISHABLE_KEY: &str = "sb_publishable_faC12u4PIEuLdnwmMXxK8g_Q_5Hy7bi";

const MAX_BODY: u64 = 4 * 1024 * 1024;
const PULL_CONVERSATIONS: usize = 50;
const PULL_MESSAGES: usize = 500;

#[derive(Clone)]
pub struct Config {
    pub url: String,
    pub key: String,
}

impl Config {
    /// Environment overrides exist for tests only (`RETINACARE_SUPABASE_URL`); HTTPS is required for
    /// anything that is not loopback.
    pub fn from_env() -> Self {
        let url = std::env::var("RETINACARE_SUPABASE_URL")
            .ok()
            .filter(|u| url_is_acceptable(u))
            .unwrap_or_else(|| DEFAULT_URL.to_string());
        Self {
            url: url.trim_end_matches('/').to_string(),
            key: PUBLISHABLE_KEY.to_string(),
        }
    }
}

pub fn url_is_acceptable(url: &str) -> bool {
    url.starts_with("https://")
        || url.starts_with("http://127.0.0.1:")
        || url.starts_with("http://localhost:")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudError {
    /// No network or the server did not answer: the app stays usable offline.
    Offline,
    /// Wrong email or password, or the account does not exist.
    BadCredentials,
    /// The access token was refused and could not be refreshed: sign in again.
    SessionExpired,
    /// The server refused the request (RLS, validation). Carries the status only, never the body.
    Rejected(u16),
    /// The reply was not what the protocol promises.
    BadResponse,
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline => write!(f, "No connection to the cloud service."),
            Self::BadCredentials => write!(f, "Email or password was not accepted."),
            Self::SessionExpired => write!(f, "The cloud session ended. Sign in again."),
            Self::Rejected(code) => {
                write!(f, "The cloud service refused the request (HTTP {code}).")
            }
            Self::BadResponse => write!(f, "The cloud service sent an unexpected reply."),
        }
    }
}

/// A signed-in session. Debug output never shows a token.
pub struct Session {
    pub access_token: Zeroizing<String>,
    pub refresh_token: Zeroizing<String>,
    pub user_id: String,
    pub email: String,
    pub expires_at: Instant,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("user_id", &self.user_id)
            .field("tokens", &"<redacted>")
            .finish()
    }
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    expires_in: u64,
    user: TokenUser,
}

#[derive(Deserialize)]
struct TokenUser {
    id: String,
    #[serde(default)]
    email: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteConversation {
    pub id: String,
    pub title: String,
    pub deleted: bool,
}

pub struct Client {
    cfg: Config,
    agent: ureq::Agent,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(6)))
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into()
}

fn map_transport(error: &ureq::Error) -> CloudError {
    match error {
        ureq::Error::Io(_)
        | ureq::Error::ConnectionFailed
        | ureq::Error::HostNotFound
        | ureq::Error::Timeout(_) => CloudError::Offline,
        _ => CloudError::BadResponse,
    }
}

impl Client {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            agent: agent(),
        }
    }

    fn session_from(
        &self,
        mut reply: ureq::http::Response<ureq::Body>,
    ) -> Result<Session, CloudError> {
        let parsed: TokenReply = reply
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_json()
            .map_err(|_| CloudError::BadResponse)?;
        if parsed.access_token.is_empty()
            || parsed.refresh_token.is_empty()
            || parsed.user.id.is_empty()
        {
            return Err(CloudError::BadResponse);
        }
        Ok(Session {
            access_token: Zeroizing::new(parsed.access_token),
            refresh_token: Zeroizing::new(parsed.refresh_token),
            user_id: parsed.user.id,
            email: parsed.user.email,
            expires_at: Instant::now()
                + Duration::from_secs(parsed.expires_in.max(60).saturating_sub(30)),
        })
    }

    pub fn sign_in(&self, email: &str, password: &str) -> Result<Session, CloudError> {
        let body = serde_json::json!({"email": email, "password": password});
        let reply = self
            .agent
            .post(&format!(
                "{}/auth/v1/token?grant_type=password",
                self.cfg.url
            ))
            .header("apikey", &self.cfg.key)
            .send_json(&body)
            .map_err(|e| map_transport(&e))?;
        match reply.status().as_u16() {
            200 => self.session_from(reply),
            400 | 401 | 422 => Err(CloudError::BadCredentials),
            code => Err(CloudError::Rejected(code)),
        }
    }

    pub fn refresh(&self, refresh_token: &str) -> Result<Session, CloudError> {
        let body = serde_json::json!({"refresh_token": refresh_token});
        let reply = self
            .agent
            .post(&format!(
                "{}/auth/v1/token?grant_type=refresh_token",
                self.cfg.url
            ))
            .header("apikey", &self.cfg.key)
            .send_json(&body)
            .map_err(|e| map_transport(&e))?;
        match reply.status().as_u16() {
            200 => self.session_from(reply),
            400 | 401 | 403 | 422 => Err(CloudError::SessionExpired),
            code => Err(CloudError::Rejected(code)),
        }
    }

    /// Best effort: the server revokes the refresh token. Failure is not an error for the caller.
    pub fn sign_out(&self, session: &Session) {
        let _ = self
            .agent
            .post(&format!("{}/auth/v1/logout", self.cfg.url))
            .header("apikey", &self.cfg.key)
            .header(
                "Authorization",
                &format!("Bearer {}", session.access_token.as_str()),
            )
            .send_empty();
    }

    fn rest(
        &self,
        session: &Session,
        method: &str,
        path_query: &str,
        prefer: Option<&str>,
        body: Option<&serde_json::Value>,
    ) -> Result<ureq::http::Response<ureq::Body>, CloudError> {
        let url = format!("{}/rest/v1/{}", self.cfg.url, path_query);
        let bearer = format!("Bearer {}", session.access_token.as_str());
        let reply = match (method, body) {
            ("GET", _) => self
                .agent
                .get(&url)
                .header("apikey", &self.cfg.key)
                .header("Authorization", &bearer)
                .call(),
            (_, Some(body)) => {
                let mut req = self
                    .agent
                    .post(&url)
                    .header("apikey", &self.cfg.key)
                    .header("Authorization", &bearer);
                if let Some(p) = prefer {
                    req = req.header("Prefer", p);
                }
                req.send_json(body)
            }
            _ => return Err(CloudError::BadResponse),
        }
        .map_err(|e| map_transport(&e))?;
        match reply.status().as_u16() {
            200 | 201 | 204 => Ok(reply),
            401 => Err(CloudError::SessionExpired),
            code => Err(CloudError::Rejected(code)),
        }
    }

    /// Create or rename a general conversation under the caller's account (idempotent by id).
    pub fn upsert_conversation(
        &self,
        session: &Session,
        id: &str,
        title: &str,
    ) -> Result<(), CloudError> {
        let body = serde_json::json!({"id": id, "title": title, "data_class": "general"});
        self.rest(
            session,
            "POST",
            "conversations?on_conflict=id",
            Some("resolution=merge-duplicates,return=minimal"),
            Some(&body),
        )
        .map(|_| ())
    }

    /// Append one message; a repeat of the same `client_msg_id` is ignored by the server.
    pub fn insert_message(
        &self,
        session: &Session,
        conversation_id: &str,
        message: &retinacare_persistence::StoredMessage,
    ) -> Result<(), CloudError> {
        let body = serde_json::json!({
            "conversation_id": conversation_id,
            "client_msg_id": message.client_msg_id,
            "role": message.role,
            "content": message.content,
        });
        self.rest(
            session,
            "POST",
            "conversation_messages?on_conflict=conversation_id,client_msg_id",
            Some("resolution=ignore-duplicates,return=minimal"),
            Some(&body),
        )
        .map(|_| ())
    }

    pub fn list_conversations(
        &self,
        session: &Session,
    ) -> Result<Vec<RemoteConversation>, CloudError> {
        let mut reply = self.rest(
            session,
            "GET",
            &format!(
                "conversations?select=id,title,deleted_at&order=updated_at.desc&limit={PULL_CONVERSATIONS}"
            ),
            None,
            None,
        )?;
        let rows: Vec<serde_json::Value> = reply
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_json()
            .map_err(|_| CloudError::BadResponse)?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                Some(RemoteConversation {
                    id: r["id"].as_str()?.to_string(),
                    title: r["title"].as_str()?.to_string(),
                    deleted: !r["deleted_at"].is_null(),
                })
            })
            .collect())
    }

    pub fn list_messages(
        &self,
        session: &Session,
        conversation_id: &str,
    ) -> Result<Vec<MessageInput>, CloudError> {
        if !retinacare_persistence::is_uuid(conversation_id) {
            return Err(CloudError::BadResponse);
        }
        let mut reply = self.rest(
            session,
            "GET",
            &format!(
                "conversation_messages?conversation_id=eq.{conversation_id}&select=client_msg_id,role,content&order=seq.asc&limit={PULL_MESSAGES}"
            ),
            None,
            None,
        )?;
        let rows: Vec<serde_json::Value> = reply
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_json()
            .map_err(|_| CloudError::BadResponse)?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                Some(MessageInput {
                    client_msg_id: r["client_msg_id"].as_str()?.to_string(),
                    role: r["role"].as_str()?.to_string(),
                    kind: "text".into(),
                    status: "completed".into(),
                    content: r["content"].as_str()?.to_string(),
                    card_json: None,
                    provider: None,
                    attachment_count: 0,
                })
            })
            .collect())
    }
}

/// Outcome of one push pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PushReport {
    pub sent: usize,
    pub failed: usize,
    pub stopped: Option<CloudError>,
}

/// Push queued messages. The first transport/auth error stops the pass (the rest stay queued, in order);
/// a refused single item is recorded and skipped so one bad row cannot block the others.
pub fn push_outbox(
    client: &Client,
    session: &Session,
    items: &[OutboxItem],
    mut mark_ok: impl FnMut(i64),
    mut mark_failed: impl FnMut(i64, &str),
) -> PushReport {
    let mut report = PushReport::default();
    let mut conversations_done: Vec<&str> = Vec::new();
    for item in items {
        // Defence in depth: the store already filters these out.
        if item.message.attachment_count != 0 || item.message.kind != "text" {
            mark_failed(item.outbox_id, "not_eligible");
            report.failed += 1;
            continue;
        }
        if !conversations_done.contains(&item.conv_uuid.as_str()) {
            match client.upsert_conversation(session, &item.conv_uuid, &item.conversation_title) {
                Ok(()) => conversations_done.push(&item.conv_uuid),
                Err(error @ (CloudError::Offline | CloudError::SessionExpired)) => {
                    report.stopped = Some(error);
                    return report;
                }
                Err(CloudError::Rejected(code)) => {
                    mark_failed(item.outbox_id, &format!("http_{code}"));
                    report.failed += 1;
                    continue;
                }
                Err(_) => {
                    mark_failed(item.outbox_id, "bad_response");
                    report.failed += 1;
                    continue;
                }
            }
        }
        match client.insert_message(session, &item.conv_uuid, &item.message) {
            Ok(()) => {
                mark_ok(item.outbox_id);
                report.sent += 1;
            }
            Err(error @ (CloudError::Offline | CloudError::SessionExpired)) => {
                report.stopped = Some(error);
                return report;
            }
            Err(CloudError::Rejected(code)) => {
                mark_failed(item.outbox_id, &format!("http_{code}"));
                report.failed += 1;
            }
            Err(_) => {
                mark_failed(item.outbox_id, "bad_response");
                report.failed += 1;
            }
        }
    }
    report
}

/// A stable per-install id for `sync_state` (never derived from hardware or the user).
pub fn new_device_id() -> String {
    new_uuid()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use retinacare_persistence::StoredMessage;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub(crate) type Log = Arc<Mutex<Vec<(String, String, String)>>>; // request line, headers, body

    /// Minimal HTTP server: answers each request with the next canned (status, body).
    pub(crate) fn server(replies: Vec<(u16, String)>) -> (String, Log) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&log);
        std::thread::spawn(move || {
            for (status, body) in replies {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buf = vec![0u8; 65536];
                let mut total = 0;
                loop {
                    let n = stream.read(&mut buf[total..]).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    total += n;
                    let text = String::from_utf8_lossy(&buf[..total]).to_string();
                    if let Some(split) = text.find("\r\n\r\n") {
                        let length = text[..split]
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        if total >= split + 4 + length {
                            break;
                        }
                    }
                }
                let text = String::from_utf8_lossy(&buf[..total]).to_string();
                let (head, body_in) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
                let mut lines = head.lines();
                let request_line = lines.next().unwrap_or("").to_string();
                seen.lock().unwrap().push((
                    request_line,
                    lines.collect::<Vec<_>>().join("\n"),
                    body_in.to_string(),
                ));
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        (url, log)
    }

    fn client(url: &str) -> Client {
        Client::new(Config {
            url: url.into(),
            key: "sb_publishable_test".into(),
        })
    }

    fn token_json() -> String {
        serde_json::json!({
            "access_token": "ACCESS-SECRET", "refresh_token": "REFRESH-SECRET", "expires_in": 3600,
            "user": {"id": "11111111-1111-4111-8111-111111111111", "email": "a@example.test"}
        })
        .to_string()
    }

    fn session() -> Session {
        Session {
            access_token: Zeroizing::new("ACCESS-SECRET".into()),
            refresh_token: Zeroizing::new("REFRESH-SECRET".into()),
            user_id: "11111111-1111-4111-8111-111111111111".into(),
            email: "a@example.test".into(),
            expires_at: Instant::now() + Duration::from_secs(3000),
        }
    }

    fn item(id: i64, conv: &str, text: &str) -> OutboxItem {
        OutboxItem {
            outbox_id: id,
            conv_uuid: conv.into(),
            conversation_title: "Title".into(),
            attempts: 0,
            message: StoredMessage {
                client_msg_id: new_uuid(),
                seq: id,
                role: "user".into(),
                kind: "text".into(),
                status: "completed".into(),
                content: text.into(),
                card_json: None,
                provider: None,
                attachment_count: 0,
                created_at: "2026-10-29T00:00:00Z".into(),
            },
        }
    }

    #[test]
    fn sign_in_sends_the_publishable_key_and_never_leaks_tokens_in_debug() {
        let (url, log) = server(vec![(200, token_json())]);
        let s = client(&url).sign_in("a@example.test", "pw-123456").unwrap();
        assert_eq!(s.user_id, "11111111-1111-4111-8111-111111111111");
        let log = log.lock().unwrap();
        assert!(log[0]
            .0
            .starts_with("POST /auth/v1/token?grant_type=password"));
        assert!(log[0]
            .1
            .to_ascii_lowercase()
            .contains("apikey: sb_publishable_test"));
        assert!(!log[0].1.contains("service_role"));
        let shown = format!("{s:?}");
        assert!(!shown.contains("SECRET") && shown.contains("redacted"));
    }

    #[test]
    fn credentials_errors_offline_and_garbage_are_distinct() {
        let (url, _) = server(vec![(400, r#"{"error":"invalid_grant"}"#.into())]);
        assert_eq!(
            client(&url).sign_in("a@b.c", "x").unwrap_err(),
            CloudError::BadCredentials
        );
        let (url, _) = server(vec![(200, "not json".into())]);
        assert_eq!(
            client(&url).sign_in("a@b.c", "x").unwrap_err(),
            CloudError::BadResponse
        );
        let (url, _) = server(vec![(
            200,
            r#"{"access_token":"","refresh_token":"r","user":{"id":"u"}}"#.into(),
        )]);
        assert_eq!(
            client(&url).sign_in("a@b.c", "x").unwrap_err(),
            CloudError::BadResponse
        );
        // Nothing listens on port 9 of loopback: reported as offline, not as bad credentials.
        assert_eq!(
            client("http://127.0.0.1:9")
                .sign_in("a@b.c", "x")
                .unwrap_err(),
            CloudError::Offline
        );
        let (url, _) = server(vec![(400, "{}".into())]);
        assert_eq!(
            client(&url).refresh("r").unwrap_err(),
            CloudError::SessionExpired
        );
    }

    #[test]
    fn push_upserts_each_conversation_once_and_marks_rows() {
        let conv = new_uuid();
        let (url, log) = server(vec![
            (201, String::new()),
            (201, String::new()),
            (201, String::new()),
        ]);
        let items = vec![item(1, &conv, "one"), item(2, &conv, "two")];
        let (mut ok, mut bad) = (Vec::new(), Vec::new());
        let report = push_outbox(
            &client(&url),
            &session(),
            &items,
            |i| ok.push(i),
            |i, e| bad.push((i, e.to_string())),
        );
        assert_eq!(
            report,
            PushReport {
                sent: 2,
                failed: 0,
                stopped: None
            }
        );
        assert_eq!(ok, vec![1, 2]);
        assert!(bad.is_empty());
        let log = log.lock().unwrap();
        assert!(log[0]
            .0
            .starts_with("POST /rest/v1/conversations?on_conflict=id"));
        assert!(log[0]
            .1
            .to_ascii_lowercase()
            .contains("authorization: bearer access-secret"));
        let first: serde_json::Value = serde_json::from_str(&log[0].2).unwrap();
        assert_eq!(first["data_class"], "general");
        assert!(log[1].0.starts_with(
            "POST /rest/v1/conversation_messages?on_conflict=conversation_id,client_msg_id"
        ));
        assert!(log[1]
            .1
            .to_ascii_lowercase()
            .contains("prefer: resolution=ignore-duplicates"));
        let second: serde_json::Value = serde_json::from_str(&log[1].2).unwrap();
        assert_eq!(second["content"], "one");
        assert!(second.get("seq").is_none(), "the server assigns the order");
        assert_eq!(log.len(), 3, "one conversation upsert, two messages");
    }

    #[test]
    fn a_refused_row_is_recorded_and_the_rest_continue() {
        let conv = new_uuid();
        let (url, _) = server(vec![
            (201, String::new()),
            (403, "{}".into()),
            (201, String::new()),
        ]);
        let items = vec![item(1, &conv, "one"), item(2, &conv, "two")];
        let (mut ok, mut bad) = (Vec::new(), Vec::new());
        let r = push_outbox(
            &client(&url),
            &session(),
            &items,
            |i| ok.push(i),
            |i, e| bad.push((i, e.to_string())),
        );
        assert_eq!((r.sent, r.failed, r.stopped), (1, 1, None));
        assert_eq!(bad, vec![(1, "http_403".to_string())]);
        assert_eq!(ok, vec![2]);
    }

    #[test]
    fn network_loss_or_expiry_stops_the_pass_and_keeps_everything_queued() {
        let conv = new_uuid();
        let items = vec![item(1, &conv, "one"), item(2, &conv, "two")];
        let (mut ok, mut bad) = (Vec::new(), Vec::new());
        let r = push_outbox(
            &client("http://127.0.0.1:9"),
            &session(),
            &items,
            |i| ok.push(i),
            |i, e| bad.push((i, e.to_string())),
        );
        assert_eq!(r.stopped, Some(CloudError::Offline));
        assert!(ok.is_empty() && bad.is_empty());
        let (url, _) = server(vec![(401, "{}".into())]);
        let r = push_outbox(&client(&url), &session(), &items, |_| {}, |_, _| {});
        assert_eq!(r.stopped, Some(CloudError::SessionExpired));
    }

    #[test]
    fn ineligible_rows_are_never_sent() {
        let conv = new_uuid();
        let mut with_file = item(1, &conv, "x");
        with_file.message.attachment_count = 1;
        let mut card = item(2, &conv, "x");
        card.message.kind = "analysis".into();
        let (url, log) = server(vec![]);
        let mut bad = Vec::new();
        let r = push_outbox(
            &client(&url),
            &session(),
            &[with_file, card],
            |_| {},
            |i, e| bad.push((i, e.to_string())),
        );
        assert_eq!((r.sent, r.failed), (0, 2));
        assert!(
            log.lock().unwrap().is_empty(),
            "no request left the process"
        );
        assert_eq!(bad[0].1, "not_eligible");
    }

    #[test]
    fn pulls_parse_rows_and_skip_malformed_ones() {
        let cid = new_uuid();
        let convs = serde_json::json!([
            {"id": cid, "title": "A", "deleted_at": null},
            {"id": cid, "title": "B", "deleted_at": "2026-01-01T00:00:00Z"},
            {"title": "no id"}
        ]);
        let msgs = serde_json::json!([
            {"client_msg_id": new_uuid(), "role": "user", "content": "hi"},
            {"role": "user"}
        ]);
        let (url, log) = server(vec![(200, convs.to_string()), (200, msgs.to_string())]);
        let c = client(&url);
        let list = c.list_conversations(&session()).unwrap();
        assert_eq!(list.len(), 2);
        assert!(!list[0].deleted && list[1].deleted);
        let m = c.list_messages(&session(), &cid).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].content, "hi");
        assert!(log.lock().unwrap()[1]
            .0
            .contains(&format!("conversation_id=eq.{cid}")));
        assert!(
            c.list_messages(&session(), "x; drop").is_err(),
            "ids are validated before use in a URL"
        );
    }

    #[test]
    fn urls_must_be_https_or_loopback() {
        assert!(url_is_acceptable("https://x.supabase.co"));
        assert!(url_is_acceptable("http://127.0.0.1:5432"));
        assert!(!url_is_acceptable("http://evil.example"));
        assert!(!url_is_acceptable("ftp://x"));
    }
}
