//! App glue for Home conversations (Phase 5): encrypted persistence, the two composer modes, live
//! analysis cards. Child module of the crate root, so it reaches the app state directly.
//!
//! Rules enforced here:
//! - Mode R never calls a language model; its replies are the real cascade events and a summary built
//!   only from the verified outcome.
//! - Mode C sends typed turns to the configured local provider only. Attachments are never sent.
//! - A message is stored only while the encrypted database is open; otherwise it stays in memory and
//!   the history page says so.

use crate::domain::{
    AnalysisCard, AttachmentKind, ChatMessage, ChatMessageStatus, ChatRequest, ChatSession,
    LocalAttachment, MessageSender,
};
use crate::engine::cascade::{CascadeOutcome, OutcomeState, SCRATCH_EXAM_BASE};
use crate::engine::ollama;
use crate::ui::composer::ComposerMode;
use crate::RetinaCareApp;
use eframe::egui;
use retinacare_persistence::{ConversationMode, MessageInput, StoredMessage};

pub const NOT_STORED: &str =
    "Not stored: the encrypted database is locked. Unlock it (Patients page) to keep this conversation.";

fn status_word(status: ChatMessageStatus) -> &'static str {
    match status {
        ChatMessageStatus::Queued => "queued",
        ChatMessageStatus::Streaming => "streaming",
        ChatMessageStatus::Completed => "completed",
        ChatMessageStatus::Cancelled => "cancelled",
        ChatMessageStatus::Failed => "failed",
    }
}

fn parse_status(word: &str) -> ChatMessageStatus {
    match word {
        "completed" => ChatMessageStatus::Completed,
        "cancelled" => ChatMessageStatus::Cancelled,
        "failed" => ChatMessageStatus::Failed,
        // A message still queued or streaming when the app closed never finished.
        _ => ChatMessageStatus::Cancelled,
    }
}

/// Remember which conversation is open so Home shows it again after a restart. The file holds only
/// the random conversation id; the content stays in the encrypted database.
pub(crate) fn remember_conversation(id: Option<&str>) {
    if cfg!(test) {
        return;
    }
    let path = crate::engine::store::data_dir().join("active_conversation");
    match id {
        Some(id) => {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
            let _ = std::fs::write(path, id);
        }
        None => {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn remembered_conversation() -> Option<String> {
    if cfg!(test) {
        return None;
    }
    std::fs::read_to_string(crate::engine::store::data_dir().join("active_conversation"))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// `HH:MM` of a stored UTC timestamp, for display only.
fn clock(iso: &str) -> String {
    iso.get(11..16).unwrap_or("").to_string()
}

/// What a stored assistant text message keeps of its paper grounding. It stays in the local encrypted
/// store only: the cloud sync path never sends `card_json`.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedSources {
    sources: Vec<crate::domain::SourceRef>,
    note: Option<String>,
}

/// `card_json` of a stored message: the analysis card, or the paper sources of a text reply.
fn aux_json(msg: &ChatMessage) -> Option<String> {
    if let Some(card) = &msg.card {
        return serde_json::to_string(card).ok();
    }
    if !msg.attachments.is_empty() {
        return serde_json::to_string(&SavedAttachments {
            attachments: msg
                .attachments
                .iter()
                .map(|a| SavedAttachment {
                    name: a.display_name.clone(),
                    kind: kind_word(&a.kind).into(),
                    path: a.source_path.to_string_lossy().into_owned(),
                    size: a.size_bytes,
                    pages: a.page_count,
                    sha256: a.sha256_hash.clone(),
                })
                .collect(),
        })
        .ok();
    }
    if msg.sources.is_empty() && msg.source_note.is_none() {
        return None;
    }
    serde_json::to_string(&SavedSources {
        sources: msg.sources.clone(),
        note: msg.source_note.clone(),
    })
    .ok()
}

/// A sent attachment as the encrypted store keeps it: where it was, never its contents or text.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedAttachment {
    name: String,
    kind: String,
    path: String,
    size: u64,
    pages: Option<usize>,
    sha256: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedAttachments {
    attachments: Vec<SavedAttachment>,
}

fn kind_word(kind: &AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::FundusImage => "image",
        AttachmentKind::PdfReport => "pdf",
        AttachmentKind::Dicom => "dicom",
    }
}

/// Rebuild an attachment from its stored reference. A file that moved or changed is still listed, with
/// the reason in its name, so the chat never pretends it can be opened.
fn attachment_from_saved(i: usize, s: SavedAttachment) -> LocalAttachment {
    let path = std::path::PathBuf::from(&s.path);
    let state = match std::fs::metadata(&path) {
        Err(_) => " - file missing",
        Ok(meta) if meta.len() != s.size => " - file changed",
        Ok(_) => match (&s.sha256, std::fs::read(&path)) {
            (Some(want), Ok(bytes)) if crate::security::compute_sha256(&bytes) != *want => {
                " - file changed"
            }
            _ => "",
        },
    };
    LocalAttachment {
        id: format!("att_{}", i + 1),
        display_name: format!("{}{state}", s.name),
        kind: match s.kind.as_str() {
            "pdf" => AttachmentKind::PdfReport,
            "dicom" => AttachmentKind::Dicom,
            _ => AttachmentKind::FundusImage,
        },
        source_path: path,
        size_bytes: s.size,
        page_count: s.pages,
        extracted_text: None,
        sha256_hash: s.sha256,
    }
}

pub fn message_from_stored(m: &StoredMessage) -> ChatMessage {
    let user = m.role == "user";
    let mut msg = if user {
        ChatMessage::new_user(
            m.client_msg_id.clone(),
            m.content.clone(),
            clock(&m.created_at),
            Vec::new(),
        )
    } else {
        ChatMessage::new_assistant(
            m.client_msg_id.clone(),
            m.content.clone(),
            clock(&m.created_at),
            parse_status(&m.status),
            None,
        )
    };
    msg.persist_id = Some(m.client_msg_id.clone());
    msg.provider = m.provider.clone();
    msg.stored_attachments = usize::try_from(m.attachment_count).unwrap_or(0);
    if user {
        if let Some(saved) = m
            .card_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<SavedAttachments>(json).ok())
        {
            msg.attachments = saved
                .attachments
                .into_iter()
                .enumerate()
                .map(|(i, s)| attachment_from_saved(i, s))
                .collect();
        }
    }
    if user && m.content.is_empty() && m.attachment_count > 0 && msg.attachments.is_empty() {
        // The file itself is not kept in the conversation; say so instead of showing an empty bubble.
        msg.text = format!(
            "[{} attachment(s) sent; the files are not kept in the conversation]",
            m.attachment_count
        );
    }
    if m.kind == "analysis" {
        msg.card = m
            .card_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<AnalysisCard>(json).ok());
        if let Some(card) = &mut msg.card {
            // An unfinished run cannot continue after a restart.
            if card.state == "running" {
                card.state = "interrupted".into();
            }
        }
    }
    if !user && m.kind == "text" {
        if let Some(saved) = m
            .card_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<SavedSources>(json).ok())
        {
            msg.sources = saved.sources;
            msg.source_note = saved.note;
        }
    }
    if m.kind == "error" || (m.status == "failed" && !user) {
        msg.status = ChatMessageStatus::Failed;
        msg.error_detail = Some(m.content.clone());
        msg.text.clear();
    }
    msg
}

/// Only numbers the backend produced, plus fixed wording. No identity, no image, no invented text.
/// The finished run as a headed list, numbers only (the same values as `grounded_summary`).
pub fn results_message(outcome: &CascadeOutcome) -> String {
    // One short reading under the card; the card above already lists every measured number.
    let mut out = format!("Analysis finished: {}.\n", outcome.state.label());
    if let Some(g) = &outcome.grading {
        out.push_str(&format!(
            "\nThe five-fold model gave technical grade {}{} (gradable probability {:.2}).",
            g.grade,
            if g.ungradable { ", ungradable" } else { "" },
            g.gradable_probability
        ));
    }
    let seen: Vec<String> = outcome
        .masks
        .iter()
        .filter(|m| m.foreground_pixels > 0)
        .map(|m| m.key.label().to_string())
        .collect();
    if !seen.is_empty() {
        out.push_str(&format!(
            "\nMasks with detected pixels: {}.",
            seen.join("; ")
        ));
    }
    if outcome.examination_id < crate::engine::cascade::SCRATCH_EXAM_BASE {
        out.push_str(&format!(
            "\nSaved as EXAM-{:03}. Lesions, vessels, explainability, findings and the report all show this same examination.",
            outcome.examination_id
        ));
    }
    out.push_str(&format!("\nRun time {:.1} s.", outcome.total_seconds));
    out.push_str("\n\nResearch only, not a diagnosis. A clinician must interpret these results.");
    out
}

pub fn grounded_summary(outcome: &CascadeOutcome) -> String {
    let mut lines = vec![format!(
        "Result: {}. Research only; not a clinical grade.",
        outcome.state.label()
    )];
    let ran = [
        ("I", outcome.screening.is_some()),
        ("II", !outcome.masks.is_empty()),
        ("III", outcome.grading.is_some()),
    ];
    let names = |on: bool| {
        let v: Vec<&str> = ran
            .iter()
            .filter(|(_, r)| *r == on)
            .map(|(n, _)| *n)
            .collect();
        if v.is_empty() {
            "none".to_string()
        } else {
            v.join(", ")
        }
    };
    lines.push(format!(
        "Stages that ran: {}. Stages that did not run: {}.",
        names(true),
        names(false)
    ));
    if let Some(s) = &outcome.screening {
        lines.push(format!(
            "Stage I screening score {:.4} against threshold {:.4}: {}.",
            s.score,
            s.threshold,
            if s.continue_cascade {
                "score above the threshold, so the image continued to Stage II and Stage III"
            } else {
                "score below the threshold, so the image stopped at grade 0"
            }
        ));
    }
    for m in &outcome.masks {
        lines.push(format!(
            "{}: {} foreground pixels ({:.3}% of the image).",
            m.key.label(),
            m.foreground_pixels,
            m.coverage_percent
        ));
    }
    if let Some(g) = &outcome.grading {
        lines.push(format!(
            "Stage III {}: technical grade {}{}; gradable probability {:.4}.",
            g.scope,
            g.grade,
            if g.ungradable { " (ungradable)" } else { "" },
            g.gradable_probability
        ));
        lines.push(format!(
            "Ordinal probabilities (CORN, P(grade > k)): {:.4}, {:.4}, {:.4}.",
            g.ordinal_probabilities[0], g.ordinal_probabilities[1], g.ordinal_probabilities[2]
        ));
    }
    if let Some(reason) = &outcome.failure {
        lines.push(format!("Failure: {reason}"));
    }
    // Devices the stages really ran on, in run order, without repeats.
    let mut devices: Vec<&str> = Vec::new();
    let used = outcome
        .screening
        .iter()
        .map(|s| s.provider.as_str())
        .chain(outcome.masks.iter().map(|m| m.provider.as_str()))
        .chain(outcome.grading.iter().map(|g| g.provider.as_str()));
    for p in used {
        if !devices.contains(&p) {
            devices.push(p);
        }
    }
    let on = if devices.is_empty() {
        String::new()
    } else {
        format!(" on {}", devices.join(" and "))
    };
    lines.push(format!(
        "Total run time {:.1} s{on}.",
        outcome.total_seconds
    ));
    lines.join("\n")
}

impl RetinaCareApp {
    /// The encrypted store, only while it is unlocked.
    pub(crate) fn open_chat_store(&self) -> Option<crate::engine::store::SharedStore> {
        self.patient_service.chat_store().filter(|s| s.is_open())
    }

    pub(crate) fn persistence_available(&self) -> bool {
        self.open_chat_store().is_some()
    }

    /// Per-frame housekeeping: load stored conversations once after unlock, keep the composer's
    /// busy flag truthful and pick up the provider check.
    pub(crate) fn chat_sync(&mut self, ctx: &egui::Context) {
        let open = self.persistence_available();
        if open && !self.chats_loaded {
            self.chats_loaded = true;
            self.reload_sessions();
            if self.active_session_id.is_none() && self.chat_messages.is_empty() {
                if let Some(id) = remembered_conversation() {
                    self.open_conversation(ctx, &id, false);
                }
            }
            // Cloud sync is optional and only starts once the encrypted store can be read and written.
            if self.cloud.is_none() && !cfg!(test) {
                if let Some(store) = self.patient_service.chat_store() {
                    let handle = crate::engine::cloud::CloudHandle::spawn(
                        store,
                        crate::engine::supabase::Config::from_env(),
                    );
                    handle.send(crate::engine::cloud::CloudCommand::Restore);
                    self.cloud = Some(handle);
                }
            }
            ctx.request_repaint();
        } else if !open && self.chats_loaded {
            // Locked again: stored conversations must not stay readable.
            self.chats_loaded = false;
            self.previous_sessions
                .retain(|s| s.id.starts_with("local_"));
            if self
                .active_session_id
                .as_deref()
                .is_some_and(|id| !id.starts_with("local_"))
            {
                self.chat_messages.clear();
                self.active_session_id = None;
                self.is_chat_docked = false;
            }
        }
        self.composer.is_busy = self.active_request_id.is_some() || self.cascade_ctl.is_busy();
        self.composer.cloud_available = self.cloud_signed_in();
        // Drag and drop onto Home attaches the files (the same checks as the file picker).
        if self.active_page == crate::AppPage::Home {
            let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect()
            });
            if !dropped.is_empty() {
                self.attach_files_from_paths(dropped);
            }
        }
        if let Some(result) = self
            .assistant_check
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
        {
            self.composer.assistant_status = Some(result);
            ctx.request_repaint();
        }
    }

    pub(crate) fn start_assistant_check(&mut self) {
        let slot = std::sync::Arc::clone(&self.assistant_check);
        let model = self.assistant_model.clone();
        std::thread::spawn(move || {
            let result = ollama::check_available(&model).map(|()| model.clone());
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(result);
            }
        });
    }

    /// Remove every saved conversation for good and return to an empty new chat.
    pub(crate) fn delete_all_chats(&mut self, ctx: &egui::Context) {
        let Some(store) = self.open_chat_store() else {
            return;
        };
        match store.with(|s| s.delete_all_conversations()) {
            Ok(n) => {
                eprintln!("deleted {n} saved conversations");
                self.chat_messages.clear();
                self.active_session_id = None;
                self.is_chat_docked = false;
                self.reset_analysis_context();
                self.previous_sessions.clear();
                self.reload_sessions();
                self.chat_notice = Some(format!("Deleted {n} saved chats."));
            }
            Err(e) => self.chat_notice = Some(format!("Could not delete the chats: {e}")),
        }
        ctx.request_repaint();
    }

    pub(crate) fn reload_sessions(&mut self) {
        let Some(store) = self.open_chat_store() else {
            return;
        };
        if self.draft_owner.is_none() {
            self.draft_owner = store
                .with(|s| s.list_patients())
                .ok()
                .and_then(|list| {
                    list.into_iter()
                        .find(|p| p.patient_code == retinacare_persistence::UNASSIGNED_CODE)
                })
                .map(|p| p.id);
        }
        let listed = store.with(|s| s.list_conversations(""));
        let Ok(list) = listed else { return };
        let mut sessions = Vec::new();
        for c in list {
            let messages = store
                .with(|s| s.load_messages(&c.conv_uuid))
                .map(|rows| rows.iter().map(message_from_stored).collect::<Vec<_>>())
                .unwrap_or_default();
            let when = c
                .updated_at
                .replace('T', " ")
                .trim_end_matches('Z')
                .to_string();
            let preview_src = if c.preview.trim().is_empty() {
                messages
                    .iter()
                    .find(|m| m.sender == MessageSender::User)
                    .map(|m| m.text.clone())
                    .unwrap_or_default()
            } else {
                c.preview.clone()
            };
            let mut session = ChatSession::new(
                c.conv_uuid.clone(),
                c.title,
                when,
                ChatSession::compress_preview_4_words(&preview_src),
                messages,
            );
            session.patient_id = c.patient_id;
            session.examination_id = c.examination_id;
            session.research = c.mode == ConversationMode::Research;
            sessions.push(session);
        }
        let local: Vec<ChatSession> = self
            .previous_sessions
            .iter()
            .filter(|s| s.id.starts_with("local_"))
            .cloned()
            .collect();
        self.previous_sessions = local.into_iter().chain(sessions).collect();
    }

    /// Open the saved conversation `id`: its stored messages, mode and linked examination. Nothing is
    /// run again; the examination comes back from its stored summary, masks and image file. A link is
    /// used only when the examination belongs to the conversation's patient.
    pub(crate) fn open_conversation(&mut self, ctx: &egui::Context, id: &str, goto_home: bool) {
        if self.active_session_id.as_deref() != Some(id) {
            self.sync_session_snapshot();
            let Some(session) = self.previous_sessions.iter().find(|s| s.id == id).cloned() else {
                return;
            };
            self.chat_messages = session.messages.clone();
            self.fill_card_owners();
            self.active_session_id = Some(session.id.clone());
            self.composer.attachments.clear();
            self.composer.prompt.clear();
            self.composer.mode = if session.research {
                ComposerMode::Research
            } else {
                ComposerMode::Assistant
            };
            self.restore_conversation_exam(ctx, &session);
        }
        self.is_chat_docked = true;
        remember_conversation(Some(id));
        if goto_home {
            self.active_page = crate::AppPage::Home;
        }
        ctx.request_repaint();
    }

    /// Cards saved before the patient was written onto them name only the examination. An examination
    /// id belongs to exactly one patient, so the owner is read back from the store, never guessed.
    fn fill_card_owners(&mut self) {
        let missing: Vec<i64> = self
            .chat_messages
            .iter()
            .filter_map(|m| m.card.as_ref())
            .filter(|c| c.saved && c.patient_id.is_none())
            .map(|c| c.examination_id)
            .collect();
        if missing.is_empty() {
            return;
        }
        let patients = self
            .patient_service
            .list_patients(&crate::engine::PatientQuery::default())
            .unwrap_or_default();
        for exam in missing {
            let owner = patients.iter().find(|p| {
                self.patient_service
                    .list_examinations(p.id)
                    .is_ok_and(|list| list.iter().any(|e| e.id == exam))
            });
            for card in self
                .chat_messages
                .iter_mut()
                .filter_map(|m| m.card.as_mut())
                .filter(|c| c.examination_id == exam && c.patient_id.is_none())
            {
                card.patient_id = owner.map(|p| p.id);
            }
        }
    }

    /// Show a saved report in the PDF viewer. The examination must belong to the patient on the card;
    /// otherwise nothing is loaded and the chat says the report is unavailable.
    pub(crate) fn open_report(&mut self, ctx: &egui::Context, patient: i64, exam: i64) {
        let owned = exam < SCRATCH_EXAM_BASE
            && self
                .patient_service
                .list_examinations(patient)
                .is_ok_and(|list| list.iter().any(|e| e.id == exam));
        if !owned {
            self.exam_view.cascade.mask_note = Some(
                "Report unavailable: the saved examination was not found for this patient.".into(),
            );
            return;
        }
        if self.active_patient_id != Some(patient) || self.active_examination_id != Some(exam) {
            self.clear_loaded_image();
            self.reset_analysis_context();
            self.active_patient_id = Some(patient);
            self.active_examination_id = Some(exam);
            self.exam_view.exam_id = format!("EXAM-{exam:03}");
            if let Ok(detail) = self.patient_service.get_patient(patient) {
                self.exam_view.patient_code = detail.patient_code.clone();
                self.patients_view_state.select_patient(Some(&detail));
            }
            self.load_exam_media(ctx, patient, exam);
            self.restore_saved_analysis(ctx, exam);
        }
        self.active_page = crate::AppPage::ReportDoc;
        ctx.request_repaint();
    }

    fn restore_conversation_exam(&mut self, ctx: &egui::Context, session: &ChatSession) {
        if !session.research {
            return;
        }
        let from_card = self
            .chat_messages
            .iter()
            .rev()
            .filter_map(|m| m.card.as_ref())
            .find(|c| c.saved && c.patient_id.is_some())
            .map(|c| (c.patient_id, c.examination_id));
        let (patient, exam) = match (session.patient_id, session.examination_id) {
            (Some(p), Some(e)) => (Some(p), Some(e)),
            (p, e) => from_card.map_or((p, e), |(cp, ce)| (p.or(cp), e.or(Some(ce)))),
        };
        // Another patient's state must not stay loaded under this conversation.
        if self.active_patient_id != patient {
            self.clear_loaded_image();
            self.reset_analysis_context();
            self.active_patient_id = patient;
            self.active_examination_id = None;
        }
        let (Some(patient), Some(exam)) = (patient, exam) else {
            return;
        };
        let owned = self
            .patient_service
            .list_examinations(patient)
            .is_ok_and(|list| list.iter().any(|e| e.id == exam));
        if !owned || exam >= SCRATCH_EXAM_BASE {
            return;
        }
        if self.active_examination_id != Some(exam) {
            self.clear_loaded_image();
            self.reset_analysis_context();
            self.active_examination_id = Some(exam);
            self.exam_view.exam_id = format!("EXAM-{exam:03}");
            if let Ok(detail) = self.patient_service.get_patient(patient) {
                self.exam_view.patient_code = detail.patient_code.clone();
                self.patients_view_state.select_patient(Some(&detail));
            }
            self.load_exam_media(ctx, patient, exam);
            self.restore_saved_analysis(ctx, exam);
            let image_ok = self
                .patient_service
                .list_examinations(patient)
                .is_ok_and(|list| {
                    list.iter()
                        .any(|e| e.id == exam && std::path::Path::new(&e.image_path).is_file())
                });
            if !image_ok {
                self.exam_view.cascade.mask_note =
                    Some("Fundus image unavailable: the stored file is missing.".into());
            }
        }
    }

    fn ensure_conversation(&mut self, first_text: &str) {
        if self.active_session_id.is_some() {
            return;
        }
        let mode = self.composer.mode;
        let title = ChatSession::auto_title(if first_text.trim().is_empty() {
            "Retinal analysis"
        } else {
            first_text
        });
        let created = self.open_chat_store().and_then(|store| {
            let (mode_db, patient, exam) = match mode {
                ComposerMode::Research => {
                    let exam = self
                        .active_examination_id
                        .filter(|e| *e < SCRATCH_EXAM_BASE);
                    (ConversationMode::Research, self.active_patient_id, exam)
                }
                ComposerMode::Assistant => (ConversationMode::Chat, None, None),
            };
            store
                .with(|s| s.create_conversation(&title, mode_db, patient, exam))
                .or_else(|_| store.with(|s| s.create_conversation(&title, mode_db, None, None)))
                .ok()
        });
        let id = created.unwrap_or_else(|| format!("local_{}", self.previous_sessions.len() + 1));
        let mut session = ChatSession::new(
            id.clone(),
            title,
            format!("Today, {}", crate::get_current_time_str()),
            ChatSession::compress_preview_4_words(first_text),
            Vec::new(),
        );
        session.research = mode == ComposerMode::Research;
        session.patient_id = if session.research {
            self.active_patient_id
        } else {
            None
        };
        self.previous_sessions.insert(0, session);
        remember_conversation(Some(&id));
        self.active_session_id = Some(id);
    }

    /// Store message `idx` of the open conversation (no-op while locked).
    fn persist_message(&mut self, idx: usize) {
        let (Some(conv), Some(store)) = (self.active_session_id.clone(), self.open_chat_store())
        else {
            return;
        };
        if conv.starts_with("local_") {
            return;
        }
        let Some(msg) = self.chat_messages.get_mut(idx) else {
            return;
        };
        let id = msg
            .persist_id
            .clone()
            .unwrap_or_else(retinacare_persistence::new_uuid);
        let (kind, content) = if msg.card.is_some() {
            ("analysis", msg.text.clone())
        } else if msg.status == ChatMessageStatus::Failed {
            ("error", msg.error_detail.clone().unwrap_or_default())
        } else {
            ("text", msg.text.clone())
        };
        let input = MessageInput {
            client_msg_id: id.clone(),
            role: if msg.sender == MessageSender::User {
                "user"
            } else {
                "assistant"
            }
            .into(),
            kind: kind.into(),
            status: status_word(msg.status).into(),
            content,
            card_json: aux_json(msg),
            provider: msg.provider.clone(),
            attachment_count: msg.attachments.len() as i64,
        };
        if store.with(|s| s.append_message(&conv, &input)).is_ok() {
            msg.persist_id = Some(id);
        }
    }

    /// Final text and status of one assistant message once it stopped changing.
    pub(crate) fn persist_final(&mut self, request_id: &str) {
        let Some(idx) = self
            .chat_messages
            .iter()
            .position(|m| m.request_id.as_deref() == Some(request_id))
        else {
            return;
        };
        self.update_stored(idx);
        self.sync_session_snapshot();
    }

    pub(crate) fn update_stored(&mut self, idx: usize) {
        let Some(store) = self.open_chat_store() else {
            return;
        };
        let Some(msg) = self.chat_messages.get(idx) else {
            return;
        };
        let Some(id) = msg.persist_id.clone() else {
            return;
        };
        let content = if msg.status == ChatMessageStatus::Failed {
            msg.error_detail.clone().unwrap_or_default()
        } else {
            msg.text.clone()
        };
        let card = aux_json(msg);
        let status = status_word(msg.status);
        let _ = store.with(|s| s.update_message(&id, &content, status, card.as_deref()));
    }

    /// Keep the in-memory history entry equal to the open conversation.
    pub(crate) fn sync_session_snapshot(&mut self) {
        let Some(active) = &self.active_session_id else {
            return;
        };
        let (messages, attachments) = (
            self.chat_messages.clone(),
            self.chat_messages.iter().map(|m| m.attachments.len()).sum(),
        );
        if let Some(session) = self.previous_sessions.iter_mut().find(|s| s.id == *active) {
            session.message_count = messages.len();
            session.attachment_count = attachments;
            session.messages = messages;
        }
    }

    fn push_message(&mut self, msg: ChatMessage) -> usize {
        self.chat_messages.push(msg);
        let idx = self.chat_messages.len() - 1;
        self.persist_message(idx);
        idx
    }

    fn assistant_notice(&mut self, text: &str, status: ChatMessageStatus, error: Option<String>) {
        let ts = crate::get_current_time_str();
        let mut msg = ChatMessage::new_assistant(
            format!("asst_{}", self.chat_messages.len() + 1),
            text,
            ts,
            status,
            None,
        );
        msg.error_detail = error;
        self.push_message(msg);
    }

    /// Entry point of the composer's send button.
    pub(crate) fn chat_submit(
        &mut self,
        ctx: &egui::Context,
        prompt: String,
        attachments: Vec<LocalAttachment>,
    ) {
        // One composer, no mode switch: an image means analysis, plain text means chat.
        let has_image = attachments
            .iter()
            .any(|a| a.kind == AttachmentKind::FundusImage);
        self.composer.mode = if has_image {
            ComposerMode::Research
        } else {
            ComposerMode::Assistant
        };
        self.ensure_conversation(&prompt);
        let ts = crate::get_current_time_str();
        let user = ChatMessage::new_user(
            format!("usr_{}", self.chat_messages.len() + 1),
            prompt.clone(),
            ts,
            attachments.clone(),
        );
        self.push_message(user);
        self.is_chat_docked = true;
        self.auto_scroll_to_bottom = true;
        // An image starts the local analysis, and the assistant then explains the finished report
        // and answers follow-ups from it. It never receives the image.
        if has_image {
            self.submit_research(ctx, &prompt, &attachments)
        } else {
            self.submit_assistant_with(&prompt, &attachments, true)
        }
        if !self.persistence_available() {
            self.chat_notice = Some(NOT_STORED.to_string());
        }
        self.sync_session_snapshot();
        ctx.request_repaint();
    }

    /// Apply the composer's sync choice to the open general chat (the store refuses anything ineligible).
    fn apply_chat_sync_choice(&mut self) {
        let (Some(conv), Some(store)) = (self.active_session_id.clone(), self.open_chat_store())
        else {
            return;
        };
        if conv.starts_with("local_") {
            return;
        }
        let want = self.composer.sync_chat && self.cloud_signed_in();
        let _ = store.with(|s| s.set_conversation_sync(&conv, want));
    }

    /// Earlier completed text turns before message `end`, oldest first (at most 12).
    pub(crate) fn chat_history_before(&self, end: usize) -> Vec<(bool, String)> {
        let mut turns: Vec<(bool, String)> = self.chat_messages
            [..end.min(self.chat_messages.len())]
            .iter()
            .rev()
            .filter(|m| {
                m.card.is_none() && m.status == ChatMessageStatus::Completed && !m.text.is_empty()
            })
            .take(12)
            .map(|m| (m.sender == MessageSender::User, m.text.clone()))
            .collect();
        turns.reverse();
        turns
    }

    /// Verified run summary of the open examination, when the user shares it or `force` is set.
    pub(crate) fn chat_results_context(&self, force: bool) -> Option<String> {
        if !(self.include_results_in_chat || force) {
            return None;
        }
        self.exam_view
            .cascade
            .outcome
            .as_ref()
            .map(grounded_summary)
    }

    /// `with_results` sends the verified run summary as context whatever the user's toggle says.
    fn submit_assistant_with(
        &mut self,
        prompt: &str,
        attachments: &[LocalAttachment],
        with_results: bool,
    ) {
        self.apply_chat_sync_choice();
        let papers: Vec<LocalAttachment> = attachments
            .iter()
            .filter(|a| a.kind == AttachmentKind::PdfReport)
            .cloned()
            .collect();
        if papers.len() != attachments.len() {
            self.assistant_notice(
                "",
                ChatMessageStatus::Failed,
                Some("The chat assistant never receives images. Switch to RetinaCare Research Analysis to analyse an image.".into()),
            );
            return;
        }
        if prompt.trim().is_empty() {
            return;
        }
        // The message just sent travels as the prompt.
        let history = self.chat_history_before(self.chat_messages.len().saturating_sub(1));
        let results_context = self.chat_results_context(with_results);
        let req_id = format!(
            "req_{}_{}",
            self.chat_messages.len(),
            crate::get_current_time_str().replace([' ', ':'], "")
        );
        let mut reply = ChatMessage::new_assistant(
            format!("asst_{}", self.chat_messages.len() + 1),
            "",
            crate::get_current_time_str(),
            ChatMessageStatus::Queued,
            Some(req_id.clone()),
        );
        reply.provider = Some(format!("{} (local Ollama)", self.assistant_model));
        self.push_message(reply);
        self.active_request_id = Some(req_id.clone());
        let request = ChatRequest {
            request_id: req_id,
            session_id: self.active_session_id.clone().unwrap_or_default(),
            prompt: prompt.to_string(),
            attachments: papers,
            model_identifier: self.assistant_model.clone(),
            history,
            results_context,
        };
        if let Err(error) = self
            .chat_service
            .submit(request, self.chat_event_tx.clone())
        {
            if let Some(m) = self.chat_messages.last_mut() {
                m.fail(error.to_string());
            }
            self.active_request_id = None;
        }
    }

    fn submit_research(
        &mut self,
        ctx: &egui::Context,
        prompt: &str,
        attachments: &[LocalAttachment],
    ) {
        let image = attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::FundusImage);
        if attachments
            .iter()
            .any(|a| a.kind == AttachmentKind::PdfReport)
        {
            self.assistant_notice(
                "Report PDFs are kept with the message only. This mode analyses retinal images; no text from the PDF is interpreted.",
                ChatMessageStatus::Completed,
                None,
            );
        }
        let Some(image) = image else {
            // No image: answer only from a verified result of the open examination, else say what to do.
            let text = match self.exam_view.cascade.outcome.as_ref().filter(|o| o.identity_verified) {
                Some(outcome) if !prompt.trim().is_empty() => format!(
                    "Verified result of the open examination (no language model is used in this mode):\n{}",
                    grounded_summary(outcome)
                ),
                _ => "Attach a retinal image (PNG or JPEG) and send to run the local Stage I to Stage III analysis. This mode does not use a language model; switch to Chat Assistant to ask general questions.".to_string(),
            };
            self.assistant_notice(&text, ChatMessageStatus::Completed, None);
            return;
        };
        if self.cascade_ctl.is_busy() {
            self.assistant_notice(
                "",
                ChatMessageStatus::Failed,
                Some(
                    "An analysis is already running. Wait for it to finish or stop it first."
                        .into(),
                ),
            );
            return;
        }
        if let Err(reason) = self.backend_service.cascade_status() {
            self.assistant_notice(
                "",
                ChatMessageStatus::Failed,
                Some(format!(
                    "The local analysis cascade is unavailable: {reason}"
                )),
            );
            return;
        }
        // The image belongs to the open patient/examination, never to the last one used.
        self.load_fundus_from_path(ctx, &image.source_path);
        if let Some(error) = self.exam_view.cascade.error.take() {
            self.assistant_notice("", ChatMessageStatus::Failed, Some(error));
            return;
        }
        self.start_analysis();
        if let Some(error) = self.exam_view.cascade.error.take() {
            self.assistant_notice("", ChatMessageStatus::Failed, Some(error));
            return;
        }
        let Some(request_id) = self.exam_view.cascade.request_id else {
            self.assistant_notice(
                "",
                ChatMessageStatus::Failed,
                Some("The analysis could not be started.".into()),
            );
            return;
        };
        let eye = match self.exam_view.laterality {
            crate::domain::EyeLaterality::OD => "OD",
            crate::domain::EyeLaterality::OS => "OS",
            crate::domain::EyeLaterality::Unspecified => "eye not set",
        };
        let card = AnalysisCard {
            request_id,
            examination_id: self.active_examination_id.unwrap_or(0),
            patient_id: self.active_patient_id,
            eye: eye.into(),
            image_name: image.display_name.clone(),
            image_sha256: image.sha256_hash.clone(),
            state: "running".into(),
            grade: None,
            ensemble: false,
            saved: false,
        };
        let mut msg = ChatMessage::new_assistant(
            format!("asst_{}", self.chat_messages.len() + 1),
            "Research analysis started on this machine. Stage results appear below as the models finish.",
            crate::get_current_time_str(),
            ChatMessageStatus::Streaming,
            None,
        );
        msg.card = Some(card);
        self.push_message(msg);
    }

    /// Called when a cascade run ended (any outcome): finalise the card of that run.
    pub(crate) fn finish_analysis_card(&mut self, request_id: u64) {
        let Some(outcome) = self.exam_view.cascade.outcome.clone() else {
            return;
        };
        if outcome.request_id != request_id {
            return;
        }
        let saved = self
            .exam_view
            .cascade
            .save_note
            .as_deref()
            .is_some_and(|n| n.contains("saved with the examination"));
        let Some(idx) = self
            .chat_messages
            .iter()
            .position(|m| m.card.as_ref().is_some_and(|c| c.request_id == request_id))
        else {
            return;
        };
        let text = grounded_summary(&outcome);
        let failed = matches!(
            outcome.state,
            OutcomeState::Failed | OutcomeState::Cancelled
        );
        // A later explicit save refreshes the card only; the results message is posted once.
        let already_done = self.chat_messages[idx]
            .card
            .as_ref()
            .is_some_and(|c| Self::completed_state(&c.state));
        let active_patient = self.active_patient_id;
        let msg = &mut self.chat_messages[idx];
        if let Some(card) = &mut msg.card {
            card.state = outcome.state.label().to_string();
            card.grade = outcome.grade;
            card.ensemble = outcome.is_ensemble();
            card.saved = saved;
            card.examination_id = outcome.examination_id;
            card.patient_id = outcome
                .patient_id
                .or(card.patient_id)
                .or(saved.then_some(active_patient).flatten());
            card.image_sha256 = outcome.image_sha256.clone();
        }
        msg.text = text;
        msg.status = if failed {
            ChatMessageStatus::Failed
        } else {
            ChatMessageStatus::Completed
        };
        if failed {
            msg.error_detail = Some(
                outcome
                    .failure
                    .clone()
                    .unwrap_or_else(|| outcome.state.label().to_string()),
            );
        }
        self.update_stored(idx);
        self.sync_session_snapshot();
        if !failed && !already_done {
            // Fixed layout from the verified numbers: a small model's prose invented meanings for them.
            self.assistant_notice(
                &results_message(&outcome),
                ChatMessageStatus::Completed,
                None,
            );
        }
    }

    fn completed_state(state: &str) -> bool {
        state.starts_with("Completed")
    }

    /// The run was dropped because the patient, examination or image changed: its card says so.
    pub(crate) fn mark_card_abandoned(&mut self, request_id: u64) {
        let Some(idx) = self
            .chat_messages
            .iter()
            .position(|m| m.card.as_ref().is_some_and(|c| c.request_id == request_id))
        else {
            return;
        };
        let msg = &mut self.chat_messages[idx];
        if let Some(card) = &mut msg.card {
            card.state = "stopped: the patient, examination or image changed".into();
        }
        msg.status = ChatMessageStatus::Cancelled;
        self.update_stored(idx);
        self.sync_session_snapshot();
    }

    /// Stop button: a running analysis stops at its next stage boundary, a chat reply at its next token.
    pub(crate) fn chat_stop(&mut self) {
        if self.cascade_ctl.is_busy() {
            self.cancel_analysis();
        }
        if let Some(id) = self.active_request_id.clone() {
            let _ = self.chat_service.cancel(&id);
            let _ = self
                .chat_event_tx
                .send(crate::domain::ChatEvent::Cancelled { request_id: id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_references_survive_a_store_round_trip() {
        let dir = std::env::temp_dir().join("rc-att-roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("paper.pdf");
        std::fs::write(&file, b"%PDF-1.4 test").unwrap();
        let att = LocalAttachment {
            id: "att_1".into(),
            display_name: "paper.pdf".into(),
            kind: AttachmentKind::PdfReport,
            source_path: file.clone(),
            size_bytes: 13,
            page_count: Some(2),
            extracted_text: Some("patient text must not be stored".into()),
            sha256_hash: Some(crate::security::compute_sha256(b"%PDF-1.4 test")),
        };
        let msg = ChatMessage::new_user("u1", "", "10:00", vec![att]);
        let json = aux_json(&msg).unwrap();
        assert!(!json.contains("patient text"));
        let stored = |card_json: Option<String>| StoredMessage {
            client_msg_id: "u1".into(),
            seq: 1,
            role: "user".into(),
            kind: "text".into(),
            status: "completed".into(),
            content: String::new(),
            card_json,
            provider: None,
            attachment_count: 1,
            created_at: "2026-10-29T10:00:00Z".into(),
        };
        let back = message_from_stored(&stored(Some(json.clone())));
        assert_eq!(back.attachments.len(), 1);
        assert_eq!(back.attachments[0].display_name, "paper.pdf");
        assert_eq!(back.attachments[0].page_count, Some(2));
        assert!(back.attachments[0].extracted_text.is_none());
        std::fs::write(&file, b"%PDF-1.4 other").unwrap();
        let changed = message_from_stored(&stored(Some(json.clone())));
        assert!(changed.attachments[0]
            .display_name
            .ends_with("file changed"));
        std::fs::remove_file(&file).unwrap();
        let gone = message_from_stored(&stored(Some(json)));
        assert!(gone.attachments[0].display_name.ends_with("file missing"));
    }

    #[test]
    fn paper_sources_survive_a_store_round_trip() {
        let mut reply = ChatMessage::new_assistant(
            "a1",
            "answer [p.pdf p.2]",
            "10:00",
            ChatMessageStatus::Completed,
            None,
        );
        reply.sources = vec![crate::domain::SourceRef {
            doc: "p.pdf".into(),
            doc_id: "abc".into(),
            page: 2,
            excerpt: "excerpt".into(),
        }];
        reply.source_note = Some("note".into());
        let stored = StoredMessage {
            client_msg_id: "a1".into(),
            seq: 1,
            role: "assistant".into(),
            kind: "text".into(),
            status: "completed".into(),
            content: reply.text.clone(),
            card_json: aux_json(&reply),
            provider: None,
            attachment_count: 0,
            created_at: "2026-10-29T10:00:00Z".into(),
        };
        let back = message_from_stored(&stored);
        assert_eq!(back.sources, reply.sources);
        assert_eq!(back.source_note.as_deref(), Some("note"));
        let mut plain = reply.clone();
        plain.sources.clear();
        plain.source_note = None;
        assert!(aux_json(&plain).is_none());
    }

    #[test]
    fn stored_unfinished_messages_come_back_cancelled_never_streaming() {
        assert_eq!(parse_status("streaming"), ChatMessageStatus::Cancelled);
        assert_eq!(parse_status("queued"), ChatMessageStatus::Cancelled);
        assert_eq!(parse_status("completed"), ChatMessageStatus::Completed);
    }

    #[test]
    fn a_running_card_is_marked_interrupted_after_restart() {
        let card = AnalysisCard {
            request_id: 7,
            examination_id: 1,
            patient_id: Some(1),
            eye: "OD".into(),
            image_name: "a.png".into(),
            image_sha256: None,
            state: "running".into(),
            grade: None,
            ensemble: false,
            saved: false,
        };
        let stored = StoredMessage {
            client_msg_id: retinacare_persistence::new_uuid(),
            seq: 1,
            role: "assistant".into(),
            kind: "analysis".into(),
            status: "streaming".into(),
            content: "x".into(),
            card_json: Some(serde_json::to_string(&card).unwrap()),
            provider: None,
            attachment_count: 0,
            created_at: "2026-10-29T10:11:12Z".into(),
        };
        let msg = message_from_stored(&stored);
        assert_eq!(msg.card.unwrap().state, "interrupted");
        assert_eq!(msg.status, ChatMessageStatus::Cancelled);
        assert_eq!(msg.timestamp, "10:11");
    }

    #[test]
    fn restored_errors_show_the_stored_reason() {
        let stored = StoredMessage {
            client_msg_id: retinacare_persistence::new_uuid(),
            seq: 2,
            role: "assistant".into(),
            kind: "error".into(),
            status: "failed".into(),
            content: "provider offline".into(),
            card_json: None,
            provider: None,
            attachment_count: 0,
            created_at: "2026-10-29T10:11:12Z".into(),
        };
        let msg = message_from_stored(&stored);
        assert_eq!(msg.status, ChatMessageStatus::Failed);
        assert_eq!(msg.error_detail.as_deref(), Some("provider offline"));
    }

    fn app() -> RetinaCareApp {
        RetinaCareApp::default()
    }

    fn image_attachment() -> LocalAttachment {
        LocalAttachment {
            id: "att_1".into(),
            display_name: "x.png".into(),
            kind: AttachmentKind::FundusImage,
            source_path: std::path::PathBuf::from("/nonexistent/x.png"),
            size_bytes: 10,
            page_count: None,
            extracted_text: None,
            sha256_hash: Some("a".repeat(64)),
        }
    }

    #[test]
    fn an_image_never_reaches_the_assistant_and_starts_the_analysis_instead() {
        let mut app = app();
        app.composer.mode = ComposerMode::Assistant;
        app.chat_submit(
            &egui::Context::default(),
            "look at this".into(),
            vec![image_attachment()],
        );
        // An image always starts the local analysis; no chat request carries it.
        assert!(
            app.active_request_id.is_none(),
            "nothing was sent to the provider"
        );
        let reply = app.chat_messages.last().unwrap();
        assert_eq!(reply.status, ChatMessageStatus::Failed);
    }

    #[test]
    fn text_without_an_image_goes_to_the_assistant_in_either_mode() {
        let mut app = app();
        app.composer.mode = ComposerMode::Research;
        app.exam_view.cascade.outcome = None;
        app.chat_submit(
            &egui::Context::default(),
            "what is the grade?".into(),
            vec![],
        );
        let reply = app.chat_messages.last().unwrap();
        assert!(reply.card.is_none(), "no analysis card without a real run");
        assert!(
            app.active_request_id.is_some() || reply.status == ChatMessageStatus::Failed,
            "the text goes to the assistant, or says why it could not"
        );
    }

    #[test]
    fn an_abandoned_run_marks_its_card_and_never_stays_running() {
        let mut app = app();
        let mut msg = ChatMessage::new_assistant(
            "a1",
            "started",
            "10:00",
            ChatMessageStatus::Streaming,
            None,
        );
        msg.card = Some(AnalysisCard {
            request_id: 41,
            examination_id: 1,
            patient_id: Some(1),
            eye: "OD".into(),
            image_name: "a.png".into(),
            image_sha256: None,
            state: "running".into(),
            grade: None,
            ensemble: false,
            saved: false,
        });
        app.chat_messages.push(msg);
        app.mark_card_abandoned(41);
        let card = app.chat_messages[0].card.as_ref().unwrap();
        assert!(card.state.starts_with("stopped"));
        assert_eq!(app.chat_messages[0].status, ChatMessageStatus::Cancelled);
        app.mark_card_abandoned(999); // unknown ids change nothing
    }

    #[test]
    fn grounded_summary_contains_only_numbers_from_the_outcome() {
        use crate::engine::cascade::{GradingRow, OutcomeState};
        let outcome = CascadeOutcome {
            request_id: 1,
            examination_id: 1,
            patient_id: Some(1),
            eye: Some("OD".into()),
            state: OutcomeState::Graded,
            headline: "h".into(),
            grade: Some(3),
            screening: None,
            masks: vec![],
            grading: Some(GradingRow {
                grade: 3,
                ungradable: false,
                scope: "five-fold ensemble".into(),
                gradable_probability: 0.5,
                ordinal_probabilities: [0.9, 0.8, 0.1],
                severity_logits: None,
                model_sha256: vec![],
                provider: "CPU".into(),
                wall_ms: 1,
                research_only: true,
            }),
            failed_stage: None,
            failure: None,
            diagnostic_only: false,
            stage_timings_ms: vec![],
            total_seconds: 1.5,
            peak_rss_mib: None,
            gpu: None,
            image_sha256: None,
            image_width: None,
            image_height: None,
            identity_verified: true,
            banner: String::new(),
        };
        let text = grounded_summary(&outcome);
        assert!(text.contains("technical grade 3") && text.contains("0.9000, 0.8000, 0.1000"));
        assert!(text.contains("Research only") && text.contains("1.5 s on CPU."));
        assert!(
            !text.to_lowercase().contains("diagnos"),
            "no diagnostic wording is generated"
        );
    }
}
