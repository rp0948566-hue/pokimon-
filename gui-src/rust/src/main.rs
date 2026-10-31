#![allow(deprecated)]
//! RetinaCare AI Screening Suite &mdash; 100% Pure Rust Native Desktop Software
//! Standalone ELF executable on Ubuntu Linux (X11 / Wayland).
//! Zero TypeScript, Zero TSX, Zero WebView &mdash; 100% Rust language.

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, Id, Layout, Margin, Pos2, Rect, RichText,
    Stroke, StrokeKind, Vec2, ViewportCommand,
};
use std::sync::Arc;

#[cfg(test)]
mod cascade_flow_tests;
mod chat_app;
mod cloud_app;
pub mod domain;
pub mod engine;
mod explain_app;
pub mod plugins;
mod science_app;
pub mod security;
pub mod ui;
pub mod ui2;

use domain::{
    AnalysisStage, AttachmentKind, ChatEvent, ChatMessage, ChatMessageStatus, ChatRequest,
    ChatSession, EyeLaterality, LocalAttachment, MessageSender, RetinalExaminationView,
    ViewPanelId,
};
#[cfg(test)]
use engine::UnavailableChatService;
use engine::{extract_pdf_document, ChatService, HierarchiRetinaManifest};
use security::{compute_sha256, scrub_image_metadata, validate_attachment_path};
use ui::{theme, ComposerAction, HomeComposer, ImageViewerState, ViewerAction};

/// Content of the right-hand panel.
#[derive(Clone)]
pub enum SidePanel {
    Source(domain::WebSite),
    File(domain::LocalAttachment),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppPage {
    Home,
    /// Six-page paper view of the report (new design only; `Report` stays the retinal viewer).
    ReportDoc,
    Report,
    /// Result summary of one examination (new design only).
    Assessment,
    Patients,
    PreviousChats,
    Timeline,
    Explainability,
    LesionAnalysis,
    VesselAnalysis,
    ResearchFindings,
    Plugins,
    RetinalImaging,
    /// Optional dual-eye page, reached from Retinal Imaging (not in the sidebar).
    Bilateral,
}

pub struct RetinaCareApp {
    window_icon_sent: bool,
    sidebar_open: bool,
    active_page: AppPage,
    active_mode: usize,
    viewer: ImageViewerState,
    composer: HomeComposer,
    report_view: ui2::report_page::ReportView,
    review_view: ui2::review_page::ReviewView,
    retinal_imaging: ui2::retinal_imaging::RetinalImagingState,
    bilateral: ui2::bilateral_assessment::BilateralState,
    console: ui2::console::Console,
    report_doc: Option<retinacare_reports::PdfDocument>,
    // Box 0: Uploaded image. Boxes 1, 2, 3 remain strictly empty until model inference.
    mode_textures: [Option<egui::TextureHandle>; 4],
    uploaded_dimensions: Option<[usize; 2]>,
    uploaded_filename: Option<String>,
    chat_messages: Vec<ChatMessage>,
    is_chat_docked: bool,
    pub analysis_stage: AnalysisStage,
    pub hierarchi_manifest: HierarchiRetinaManifest,
    pub previous_sessions: Vec<ChatSession>,
    pub active_session_id: Option<String>,
    pub plugins_state: plugins::PluginsState,
    chat_event_rx: Option<std::sync::mpsc::Receiver<ChatEvent>>,
    chat_event_tx: std::sync::mpsc::Sender<ChatEvent>,
    active_request_id: Option<String>,
    auto_scroll_to_bottom: bool,
    pub exam_view: RetinalExaminationView,
    pub backend_service: std::sync::Arc<dyn crate::engine::MedicalInferenceService + Send + Sync>,
    pub backend_readiness: crate::engine::BackendReadiness,
    pub active_screening_request: Option<crate::engine::RetinalAnalysisRequest>,
    pub stage1_result: Option<crate::engine::Stage1ScreeningResult>,
    pub stage1_error: Option<String>,
    pub pdf_export_service: std::sync::Arc<dyn crate::engine::PdfExportService + Send + Sync>,
    pub audit_ledger_service: std::sync::Arc<dyn crate::engine::AuditLedgerService + Send + Sync>,
    pub export_state: crate::engine::ExportState,
    pub export_event_rx: Option<std::sync::mpsc::Receiver<crate::engine::ExportEvent>>,
    pub session_export_status: Option<String>,
    pub cached_audit_events: Vec<crate::engine::AuditEvent>,
    /// Every ledger entry, for the Timeline's report-export events. Refreshed when that page opens.
    pub all_audit_events: Vec<crate::engine::AuditEvent>,
    all_audit_count: Option<usize>,
    pub patient_service: std::sync::Arc<dyn crate::engine::PatientService + Send + Sync>,
    pub patients_view_state: ui::PatientsViewState,
    pub history_view_state: crate::domain::HistoryViewState,
    chats_view: ui2::chats_page::ChatsView,
    pub timeline_view_state: crate::domain::TimelineViewState,
    pub active_patient_id: Option<i64>,
    pub active_examination_id: Option<i64>,
    /// Single owner of the cascade request identity, stale-event rejection and queue discipline.
    cascade_ctl: crate::engine::cascade::CascadeController,
    /// Monotonic suffix for unsaved scratch examinations (ids >= SCRATCH_EXAM_BASE, never persisted).
    scratch_seq: i64,
    gpu_sampler: Option<crate::engine::gpu::GpuSampler>,
    /// Completed, verified run of an image that is not attached to a saved examination, kept so the user can save it explicitly.
    unsaved_run: Option<(
        crate::engine::cascade::JobIdentity,
        crate::engine::cascade::AnalysisReport,
    )>,
    save_dialog: Option<ui::save_exam::SaveDialog>,
    /// Id of the reserved "Unassigned" owner of draft examinations, once known.
    draft_owner: Option<i64>,
    /// An auto-explain was skipped because the worker was busy; start it when the worker is free.
    explain_auto_wanted: bool,
    /// Grad-CAM failed and the occlusion fallback is running; stops a second fallback.
    explain_fell_back: bool,
    /// Chat send held back until the user says which eye the attached image(s) show.
    eye_choice: Option<EyeChoice>,
    pub explain: ui::explain::ExplainState,
    explain_service: crate::engine::explain::ExplainService,
    /// Stored conversations were read from the unlocked database (reset when it locks again).
    chats_loaded: bool,
    chat_service: Arc<dyn ChatService>,
    assistant_model: String,
    assistant_check: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    /// Opt-in: share the verified result summary (no identity, no pixels) with the local chat model.
    include_results_in_chat: bool,
    chat_notice: Option<String>,
    /// Paper passage opened from a source chip.
    source_view: Option<crate::domain::SourceRef>,
    chat_style: ui::markdown::ChatStyle,
    /// Right-hand panel (45% of the window by default; the reader drags it between 25% and 75%).
    side: Option<SidePanel>,
    side_frac: f32,
    /// `Some(true)` opens sources in the panel, `Some(false)` in the system browser; `None` asks.
    open_pref: Option<bool>,
    pending_site: Option<domain::WebSite>,
    side_tex: Option<(std::path::PathBuf, egui::TextureHandle)>,
    side_text: Option<(std::path::PathBuf, String)>,
    science: ui::science::ScienceState,
    science_ch: science_app::ScienceChannel,
    cloud: Option<engine::cloud::CloudHandle>,
    cloud_state: engine::cloud::CloudState,
    cloud_note: Option<String>,
    cloud_window_open: bool,
    cloud_email: String,
    cloud_password: String,
}

/// The operating-system user the app runs as: the only actor identity the app genuinely knows.
impl RetinaCareApp {
    /// Chrome for the approved design: inset shell, window bar, collapsible sidebar. Returns (body rect, sidebar width).
    fn chrome_v2(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> (Rect, f32) {
        use crate::ui2::shell::{self, Icon, NavItem};
        const PAGES: [AppPage; 12] = [
            AppPage::Home,
            AppPage::ReportDoc,
            AppPage::Patients,
            AppPage::PreviousChats,
            AppPage::Timeline,
            AppPage::Explainability,
            AppPage::LesionAnalysis,
            AppPage::VesselAnalysis,
            AppPage::ResearchFindings,
            AppPage::Report,
            AppPage::RetinalImaging,
            AppPage::Assessment,
        ];
        let nav = [
            NavItem {
                label: "Home",
                icon: Icon::Home,
            },
            NavItem {
                label: "Report",
                icon: Icon::Report,
            },
            NavItem {
                label: "Patients",
                icon: Icon::Patients,
            },
            NavItem {
                label: "Previous Chats",
                icon: Icon::Chats,
            },
            NavItem {
                label: "Timeline",
                icon: Icon::Timeline,
            },
            NavItem {
                label: "Explainability",
                icon: Icon::Explain,
            },
            NavItem {
                label: "Lesion Analysis",
                icon: Icon::Lesion,
            },
            NavItem {
                label: "Vessel Analysis",
                icon: Icon::Vessel,
            },
            NavItem {
                label: "Research Findings",
                icon: Icon::Findings,
            },
            NavItem {
                label: "Retinal Review",
                icon: Icon::Eye,
            },
            NavItem {
                label: "Retinal Imaging",
                icon: Icon::Imaging,
            },
            NavItem {
                label: "Clinical Assessment",
                icon: Icon::Clip,
            },
        ];
        let mode = crate::ui2::theme::mode(ctx);
        crate::ui2::theme::apply_visuals(ctx, mode);
        self.console.show(
            ctx,
            mode.palette(),
            ctx.content_rect().shrink(crate::ui2::shell::OUTER_GAP),
        );
        // The Bilateral page belongs to Retinal Imaging in the sidebar.
        let shown_page = if self.active_page == AppPage::Bilateral {
            AppPage::RetinalImaging
        } else {
            self.active_page
        };
        let active = PAGES
            .iter()
            .position(|p| *p == shown_page)
            .unwrap_or(0);
        let store = if self.patient_service.unlock_status()
            == Some(crate::engine::store::UnlockStatus::Open)
        {
            "store unlocked"
        } else {
            "store locked"
        };
        let status = match self.backend_service.readiness() {
            crate::engine::BackendReadiness::Ready { provider, .. } => {
                ("Backend ready", format!("{provider} · {store}"))
            }
            crate::engine::BackendReadiness::Failed { .. } => ("Backend failed", store.to_string()),
            _ => ("Backend starting", store.to_string()),
        };
        // A model that wanted CUDA but runs on the CPU is shown here with its stage and reason.
        let fallback = retinacare_ai_bridge::device::gpu_fallbacks().pop();
        let status = match &fallback {
            Some(msg) => ("CPU fallback", msg.clone()),
            None => (status.0, status.1),
        };
        let out = shell::chrome(
            ui,
            ctx,
            mode.palette(),
            &mut self.sidebar_open,
            &nav,
            active,
            (status.0, status.1.as_str()),
        );
        if let Some(i) = out.clicked {
            // Home only returns to the open conversation: nothing is created or cleared.
            if PAGES[i] == AppPage::PreviousChats {
                self.reload_sessions();
            }
            if PAGES[i] == AppPage::Patients {
                // The sidebar always lands on the directory, not on a profile left open earlier.
                self.patients_view_state.current_subroute =
                    crate::domain::PatientSubroute::Directory;
            }
            self.active_page = PAGES[i];
            ctx.request_repaint();
        }
        if out.new_clicked {
            self.chat_messages.clear();
            self.active_session_id = None;
            self.reset_analysis_context();
            self.forget_exam_context();
            self.is_chat_docked = false;
            self.composer = HomeComposer::default();
            chat_app::remember_conversation(None);
            self.active_page = AppPage::Home;
            crate::ui2::console::log("new session");
            ctx.request_repaint();
        }
        if out.theme_toggled {
            crate::ui2::theme::set_mode(ctx, mode.toggled());
        }
        (out.body_rect, out.sidebar_width)
    }
}

fn current_operator() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "local user".to_string())
}

fn seed_default_sessions() -> Vec<ChatSession> {
    let seeds = [
        ("sess_01", "OD Screening - Microaneurysms", "Today, 02:45 PM", "14 microaneurysms, hard exudates",
         "Assess right fundus for microaneurysms and NPDR progression.",
         "### Retinal Feature Analysis\n\n• **Microvascular Integrity**: 14 distinct microaneurysms mapped in the posterior pole.\n• **Classification**: Moderate NPDR indicators present.\n• **Recommendation**: Comprehensive macular OCT recommended to evaluate retinal thickening."),
        ("sess_02", "Bilateral Macular Assessment", "Yesterday, 11:20 AM", "Bilateral macula stable dry",
         "Review bilateral macular scans for edema or neovascularization.",
         "### Macular Status Evaluation\n\n• **Foveal Contour**: Intact bilaterally with sharp reflex.\n• **Fluid Accumulation**: Zero intraretinal cystoid spaces or subretinal fluid.\n• **Prognosis**: Stable dry macula."),
        ("sess_03", "Follow-up OCT & Fundus Compare", "Oct 25, 2026 • 09:15 AM", "Macular thickness reduced 18%",
         "Compare baseline scan against current follow-up OCT.",
         "### Comparative Trajectory\n\n• **Central Subfield Thickness**: Decreased by 42 µm (-18%).\n• **Therapeutic Response**: Positive response to ongoing treatment regimen.\n• **Next Assessment**: Scheduled for 8-week interval."),
        ("sess_04", "Glaucoma Cup-to-Disc Ratio", "Oct 22, 2026 • 04:30 PM", "Vertical CDR 0.44 normal",
         "Calculate vertical and horizontal cup-to-disc ratio for OS.",
         "### Optic Nerve Head Metrics\n\n• **Vertical CDR**: 0.44 (normal physiological range <0.50).\n• **ISNT Rule**: Followed along inferior and superior rim sectors.\n• **Status**: Low risk of glaucomatous optic neuropathy."),
        ("sess_05", "Hypertensive Retinopathy Review", "Oct 18, 2026 • 01:10 PM", "Mild focal arteriolar narrowing",
         "Screen for arteriovenous nicking and copper wiring in superior temporal quadrant.",
         "### Vascular Architecture Findings\n\n• **AV Caliber Ratio**: 2:3 (mild generalized arteriolar attenuation).\n• **Crossing Defects**: Minor Salus sign at superotemporal bifurcation.\n• **Classification**: Grade 1 Hypertensive Retinopathy without acute exudation."),
    ];

    seeds
        .into_iter()
        .enumerate()
        .map(|(i, (id, title, ts, prev, user_msg, bot_msg))| {
            let msg_time = ts.split('•').next_back().unwrap_or(ts).trim();
            ChatSession::new(
                id,
                title,
                ts,
                prev,
                vec![
                    ChatMessage::new_user(format!("m{}", i * 2 + 1), user_msg, msg_time, vec![]),
                    ChatMessage::new_assistant(
                        format!("m{}", i * 2 + 2),
                        bot_msg,
                        msg_time,
                        ChatMessageStatus::Completed,
                        None,
                    ),
                ],
            )
        })
        .collect()
}

impl Default for RetinaCareApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        // Seeded conversations are demo content: tests and explicit dev fixtures only.
        let sessions = if cfg!(test) || std::env::var("RETINACARE_DEV_FIXTURES").is_ok() {
            seed_default_sessions()
        } else {
            Vec::new()
        };
        let demo_mode = std::env::var("RETINACARE_DEMO_SESSION").is_ok();
        let (initial_messages, initial_session_id, is_docked) = if demo_mode && !sessions.is_empty()
        {
            let s = &sessions[0];
            (s.messages.clone(), Some(s.id.clone()), true)
        } else {
            (Vec::new(), None, false)
        };

        let _manifest_path = crate::engine::discover_default_manifest();
        #[cfg(test)]
        let backend_service: std::sync::Arc<
            dyn crate::engine::MedicalInferenceService + Send + Sync,
        > = std::sync::Arc::new(crate::engine::MockMedicalInferenceService::new_ready());

        #[cfg(not(test))]
        let backend_service: std::sync::Arc<
            dyn crate::engine::MedicalInferenceService + Send + Sync,
        > = if std::env::var("RETINACARE_MOCK_BACKEND").is_ok() {
            std::sync::Arc::new(crate::engine::MockMedicalInferenceService::new_ready())
        } else if std::env::var("RETINACARE_BACKEND_UNAVAILABLE").is_ok() {
            std::sync::Arc::new(crate::engine::MockMedicalInferenceService::new_unavailable(
                "CUDA missing; driver not configured",
            ))
        } else {
            std::sync::Arc::new(crate::engine::LiveCodexBackendService::new(_manifest_path))
        };
        let backend_readiness = backend_service.readiness();

        #[cfg(test)]
        let pdf_export_service: std::sync::Arc<
            dyn crate::engine::PdfExportService + Send + Sync,
        > = std::sync::Arc::new(crate::engine::MockPdfExportService::default());

        #[cfg(not(test))]
        let pdf_export_service: std::sync::Arc<
            dyn crate::engine::PdfExportService + Send + Sync,
        > = if std::env::var("RETINACARE_MOCK_PDF").is_ok() {
            std::sync::Arc::new(crate::engine::MockPdfExportService::default())
        } else {
            std::sync::Arc::new(crate::engine::LiveCodexPdfService::default())
        };

        #[cfg(test)]
        let patient_service: std::sync::Arc<
            dyn crate::engine::PatientService + Send + Sync,
        > = std::sync::Arc::new(crate::engine::MockPatientService::with_fixtures());

        #[cfg(not(test))]
        let (patient_service, live_store): (
            std::sync::Arc<dyn crate::engine::PatientService + Send + Sync>,
            Option<crate::engine::store::SharedStore>,
        ) = if std::env::var("RETINACARE_DEV_FIXTURES").is_ok() {
            (
                std::sync::Arc::new(crate::engine::MockPatientService::with_fixtures()),
                None,
            )
        } else {
            let live = std::sync::Arc::new(crate::engine::LiveCodexPatientService::new());
            let store = live.store();
            store.auto_unlock();
            (live, Some(store))
        };

        #[cfg(test)]
        let audit_ledger_service: std::sync::Arc<
            dyn crate::engine::AuditLedgerService + Send + Sync,
        > = std::sync::Arc::new(crate::engine::MockAuditLedgerService::with_sample_events());

        // Dev fixtures never mix with the real ledger: they get their own in-memory one.
        #[cfg(not(test))]
        let audit_ledger_service: std::sync::Arc<
            dyn crate::engine::AuditLedgerService + Send + Sync,
        > = match live_store {
            Some(store) if std::env::var("RETINACARE_MOCK_AUDIT").is_err() => {
                std::sync::Arc::new(crate::engine::LiveCodexAuditService::with_store(store))
            }
            _ => std::sync::Arc::new(crate::engine::MockAuditLedgerService::default()),
        };

        let cached_audit_events = audit_ledger_service.query_events(None).unwrap_or_default();

        Self {
            window_icon_sent: false,
            sidebar_open: true,
            active_page: AppPage::Home,
            active_mode: 0,
            viewer: ImageViewerState::default(),
            composer: HomeComposer::default(),
            report_view: Default::default(),
            review_view: Default::default(),
            retinal_imaging: Default::default(),
            bilateral: Default::default(),
            console: Default::default(),
            report_doc: None,
            mode_textures: [None, None, None, None],
            uploaded_dimensions: None,
            uploaded_filename: None,
            chat_messages: initial_messages,
            is_chat_docked: is_docked,
            analysis_stage: AnalysisStage::Idle,
            hierarchi_manifest: HierarchiRetinaManifest::load_audit(),
            previous_sessions: sessions,
            active_session_id: initial_session_id,
            plugins_state: plugins::PluginsState::default(),
            chat_event_rx: Some(rx),
            chat_event_tx: tx,
            active_request_id: None,
            auto_scroll_to_bottom: true,
            exam_view: RetinalExaminationView::default(),
            backend_service,
            backend_readiness,
            active_screening_request: None,
            stage1_result: None,
            stage1_error: None,
            pdf_export_service,
            audit_ledger_service,
            export_state: crate::engine::ExportState::Idle,
            export_event_rx: None,
            session_export_status: None,
            cached_audit_events,
            all_audit_events: Vec::new(),
            all_audit_count: None,
            patient_service,
            patients_view_state: ui::PatientsViewState::default(),
            history_view_state: crate::domain::HistoryViewState::default(),
            chats_view: ui2::chats_page::ChatsView::default(),
            timeline_view_state: crate::domain::TimelineViewState::default(),
            active_patient_id: None,
            active_examination_id: None,
            cascade_ctl: crate::engine::cascade::CascadeController::new(),
            scratch_seq: 0,
            gpu_sampler: None,
            unsaved_run: None,
            save_dialog: None,
            draft_owner: None,
            explain_auto_wanted: false,
            explain_fell_back: false,
            eye_choice: None,
            explain: ui::explain::ExplainState::default(),
            explain_service: {
                explain_app::start_gradcam_probe();
                crate::engine::explain::ExplainService::default()
            },
            chats_loaded: false,
            #[cfg(test)]
            chat_service: Arc::new(UnavailableChatService),
            #[cfg(not(test))]
            chat_service: Arc::new(crate::engine::ollama::OllamaChatService::default()),
            assistant_model: std::env::var("RETINACARE_OLLAMA_MODEL")
                .ok()
                .filter(|m| !m.trim().is_empty() && m.len() < 100)
                .unwrap_or_else(|| crate::engine::ollama::DEFAULT_MODEL.to_string()),
            assistant_check: Arc::new(std::sync::Mutex::new(None)),
            include_results_in_chat: false,
            chat_notice: None,
            source_view: None,
            chat_style: ui::markdown::ChatStyle::default(),
            side: None,
            side_frac: 0.45,
            open_pref: None,
            pending_site: None,
            side_tex: None,
            side_text: None,
            science: ui::science::ScienceState::default(),
            science_ch: science_app::ScienceChannel::default(),
            cloud: None,
            cloud_state: engine::cloud::CloudState::SignedOut,
            cloud_note: None,
            cloud_window_open: false,
            cloud_email: String::new(),
            cloud_password: String::new(),
        }
    }
}

#[allow(dead_code)]
const MODES: [(&str, &str); 4] = [
    ("Original Image", "OD - Right Eye"),
    ("AI Lesion Detection", "HSMoE-AUNet • Blocked"),
    ("Vessel Analysis", "SwinHRUNet++ • Blocked"),
    ("Grad-CAM / LG-DRG", "LG-DRG Grade • Blocked"),
];

/// Formats standard time without extra external dependencies
fn get_current_time_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
        let secs = dur.as_secs();
        let minute = (secs / 60) % 60;
        let hour24 = (secs / 3600) % 24;
        let (hour12, am_pm) = if hour24 == 0 {
            (12, "AM")
        } else if hour24 < 12 {
            (hour24, "AM")
        } else if hour24 == 12 {
            (12, "PM")
        } else {
            (hour24 - 12, "PM")
        };
        format!("{:02}:{:02} {}", hour12, minute, am_pm)
    } else {
        "12:00 PM".to_owned()
    }
}

impl RetinaCareApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.console.open = std::env::var_os("RETINACARE_CONSOLE").is_some();
        app.start_assistant_check();

        if let Ok(page) = std::env::var("RETINACARE_PAGE") {
            match page.as_str() {
                "report" => app.active_page = AppPage::Report,
                "assessment" => app.active_page = AppPage::Assessment,
                "paper" => app.active_page = AppPage::ReportDoc,
                "patients" | "directory" => app.active_page = AppPage::Patients,
                "chats" => app.active_page = AppPage::PreviousChats,
                "timeline" => app.active_page = AppPage::Timeline,
                "explain" | "explainability" => app.active_page = AppPage::Explainability,
                "lesion" | "lesions" => app.active_page = AppPage::LesionAnalysis,
                "vessel" | "vessels" => app.active_page = AppPage::VesselAnalysis,
                "findings" | "research" => app.active_page = AppPage::ResearchFindings,
                "plugins" => app.active_page = AppPage::Plugins,
                "imaging" => app.active_page = AppPage::RetinalImaging,
                _ => app.active_page = AppPage::Home,
            }
        }

        if let Ok(id_str) = std::env::var("RETINACARE_SELECT_PATIENT") {
            if let Ok(id) = id_str.parse::<i64>() {
                if let Ok(detail) = app.patient_service.get_patient(id) {
                    app.patients_view_state.select_patient(Some(&detail));
                    app.active_patient_id = Some(id);
                    app.exam_view.patient_code = detail.patient_code;
                    app.patients_view_state.current_subroute =
                        crate::domain::PatientSubroute::Profile { patient_id: id };
                }
            }
        }

        if let Ok(sub) = std::env::var("RETINACARE_SUBROUTE") {
            match sub.as_str() {
                "directory" => {
                    app.patients_view_state.current_subroute =
                        crate::domain::PatientSubroute::Directory;
                }
                "profile" => {
                    let pid = app.active_patient_id.unwrap_or(1);
                    app.patients_view_state.current_subroute =
                        crate::domain::PatientSubroute::Profile { patient_id: pid };
                }
                "exam_detail" => {
                    let pid = app.active_patient_id.unwrap_or(1);
                    let eid = app.active_examination_id.unwrap_or(1);
                    app.patients_view_state.current_subroute =
                        crate::domain::PatientSubroute::ExamDetail {
                            patient_id: pid,
                            examination_id: eid,
                        };
                }
                _ => {}
            }
        }

        if let Ok(rep_sub) = std::env::var("RETINACARE_REPORT_SUBROUTE") {
            match rep_sub.as_str() {
                "quad" => {
                    app.exam_view.current_subroute = crate::domain::ReportSubroute::QuadViewer;
                    app.exam_view.report_view_mode = false;
                }
                "report" | "overview" => {
                    app.exam_view.current_subroute = crate::domain::ReportSubroute::ReportOverview;
                    app.exam_view.report_view_mode = true;
                }
                "pdf_export" => {
                    app.exam_view.current_subroute = crate::domain::ReportSubroute::PdfExport;
                }
                "session_export" => {
                    app.exam_view.current_subroute = crate::domain::ReportSubroute::SessionExport;
                }
                "audit_history" => {
                    app.exam_view.current_subroute = crate::domain::ReportSubroute::AuditHistory;
                }
                _ => {}
            }
        }

        if let Ok(query) = std::env::var("RETINACARE_SEARCH_QUERY") {
            app.patients_view_state.search_query = query;
        }

        if let Ok(hist_sub) = std::env::var("RETINACARE_HISTORY_SUBROUTE") {
            if hist_sub == "detail" {
                let id = if !app.previous_sessions.is_empty() {
                    app.previous_sessions[0].id.clone()
                } else {
                    "sess_01".to_string()
                };
                app.history_view_state.current_subroute = crate::domain::HistorySubroute::Detail {
                    conversation_id: id,
                };
            }
        }

        if let Ok(tl_sub) = std::env::var("RETINACARE_TIMELINE_SUBROUTE") {
            if tl_sub == "detail" {
                app.timeline_view_state.current_subroute =
                    crate::domain::TimelineSubroute::EventDetail {
                        event_id: "ev_cap_1".to_string(),
                    };
            }
        }

        if let Ok(ev_id) = std::env::var("RETINACARE_SELECT_EVENT") {
            app.timeline_view_state.selected_event_id = Some(ev_id);
        }

        if let Ok(img_path) = std::env::var("RETINACARE_LOAD_IMAGE") {
            let path = std::path::PathBuf::from(img_path);
            if path.exists() {
                app.load_fundus_from_path(&cc.egui_ctx, &path);
            }
        }

        if let Ok(zoom_str) = std::env::var("RETINACARE_ZOOM") {
            if let Ok(z) = zoom_str.parse::<f32>() {
                for state in &mut app.exam_view.panel_states {
                    state.zoom = z;
                }
                app.viewer.zoom = z;
            }
        }

        if let Ok(pan_str) = std::env::var("RETINACARE_PAN") {
            let parts: Vec<&str> = pan_str.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(px), Ok(py)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                    for state in &mut app.exam_view.panel_states {
                        state.pan = [px, py];
                    }
                    app.viewer.pan = Vec2::new(px, py);
                }
            }
        }

        if let Ok(expand_str) = std::env::var("RETINACARE_EXPAND_PANEL") {
            match expand_str.as_str() {
                "0" | "original" => app.exam_view.expanded_panel = Some(ViewPanelId::Original),
                "1" | "lesions" => app.exam_view.expanded_panel = Some(ViewPanelId::Lesions),
                "2" | "vessels" => app.exam_view.expanded_panel = Some(ViewPanelId::Vessels),
                "3" | "explain" => app.exam_view.expanded_panel = Some(ViewPanelId::Explainability),
                _ => {}
            }
        }

        if std::env::var("RETINACARE_MOCK_RESULT").is_ok() {
            app.stage1_result = Some(crate::engine::Stage1ScreeningResult {
                request_id: "req-verified-001".to_string(),
                examination_id: app.exam_view.exam_id.clone(),
                model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
                model_version: "1.0.0".to_string(),
                model_sha256: "9ad51e1f428dc59ab49f1f49e8f68a84cd23d8fd4c2c461b45b06bcc726880ac".to_string(),
                provider: "CPU Execution Provider".to_string(),
                raw_logit: 1.2954,
                screening_score: 0.7851,
                threshold: crate::engine::STAGE1_GATE_THRESHOLD,
                decision: crate::engine::Stage1GateDecision::ContinueCascade,
                elapsed_ms: 184,
                preprocessing_ms: 42,
                disclaimer: "Research and screening triage prototype only. Not certified for clinical diagnosis. Comprehensive dilated ophthalmological examination required for definitive care.".to_string(),
            });
        }

        if let Ok(err) = std::env::var("RETINACARE_MOCK_ERROR") {
            app.stage1_error = Some(err);
        }

        if std::env::var("RETINACARE_MOCK_RUNNING").is_ok() {
            app.backend_readiness = crate::engine::BackendReadiness::Processing {
                request_id: "req-running-001".to_string(),
            };
        }

        if std::env::var("RETINACARE_REPORT_VIEW").is_ok() {
            app.exam_view.report_view_mode = true;
            app.exam_view.current_subroute = crate::domain::ReportSubroute::ReportOverview;
        }

        if std::env::var("RETINACARE_EXPORT_DIALOG").is_ok() {
            app.exam_view.export_dialog_open = true;
        }

        if std::env::var("RETINACARE_AUDIT_LEDGER").is_ok() {
            app.exam_view.audit_ledger_open = true;
        }

        app
    }

    /// Attaches files from a path list to the composer
    fn attach_files_from_paths(&mut self, paths: Vec<std::path::PathBuf>) {
        for path in paths {
            let valid_path = match validate_attachment_path(&path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("Attachment blocked by security policy: {err}");
                    self.chat_notice = Some(format!("Attachment rejected: {err}"));
                    continue;
                }
            };
            let ext = valid_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let file_name = valid_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "document".to_owned());
            // Download names can be 100+ random characters; keep the ends so the extension stays visible.
            let file_name = crate::ui::explain::ellipsize(&file_name, 40);
            let size_bytes = std::fs::metadata(&valid_path).map(|m| m.len()).unwrap_or(0);

            let file_bytes = std::fs::read(&valid_path).unwrap_or_default();
            let sha256_hash = Some(compute_sha256(&file_bytes));

            if ext == "pdf" {
                match extract_pdf_document(&valid_path) {
                    Ok(res) => {
                        self.composer.attachments.push(LocalAttachment {
                            id: format!("att_{}", self.composer.attachments.len() + 1),
                            display_name: file_name,
                            kind: AttachmentKind::PdfReport,
                            source_path: valid_path,
                            size_bytes,
                            page_count: Some(res.total_pages),
                            extracted_text: Some(res.combined_summary),
                            sha256_hash,
                        });
                    }
                    Err(err) => {
                        eprintln!("PDF parse notice: {err}");
                        self.composer.attachments.push(LocalAttachment {
                            id: format!("att_{}", self.composer.attachments.len() + 1),
                            display_name: file_name,
                            kind: AttachmentKind::PdfReport,
                            source_path: valid_path,
                            size_bytes,
                            page_count: None,
                            extracted_text: Some(format!("[Scanned document: {err}]")),
                            sha256_hash,
                        });
                    }
                }
            } else {
                // At most two photographs (one per eye): a third replaces the oldest.
                let images = self
                    .composer
                    .attachments
                    .iter()
                    .filter(|a| a.kind == AttachmentKind::FundusImage)
                    .count();
                if images >= 2 {
                    if let Some(i) = self
                        .composer
                        .attachments
                        .iter()
                        .position(|a| a.kind == AttachmentKind::FundusImage)
                    {
                        self.composer.attachments.remove(i);
                    }
                }
                self.composer.attachments.push(LocalAttachment {
                    id: format!("att_{}", self.composer.attachments.len() + 1),
                    display_name: file_name,
                    kind: AttachmentKind::FundusImage,
                    source_path: valid_path,
                    size_bytes,
                    page_count: None,
                    extracted_text: None,
                    sha256_hash,
                });
            }
        }
    }

    /// Attaches fundus or retinal images
    fn pick_and_attach_images(&mut self) {
        // Automation hook (like RETINACARE_PDF_DEST): attach these files instead of opening the dialog.
        if let Some(list) = std::env::var_os("RETINACARE_AUTOPICK_IMAGE") {
            let paths = std::env::split_paths(&list).collect::<Vec<_>>();
            self.attach_files_from_paths(paths);
            return;
        }
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Medical Images (PNG/JPG)",
                &["png", "jpg", "jpeg", "webp", "bmp"],
            )
            .pick_file()
            .map(|p| vec![p])
        {
            self.attach_files_from_paths(paths);
        }
    }

    /// Attaches PDF clinical reports
    fn pick_and_attach_pdfs(&mut self) {
        // Automation hook, same idea as RETINACARE_AUTOPICK_IMAGE.
        if let Some(list) = std::env::var_os("RETINACARE_AUTOPICK_PDF") {
            self.attach_files_from_paths(std::env::split_paths(&list).collect());
            return;
        }
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("PDF Clinical Reports", &["pdf"])
            .pick_files()
        {
            self.attach_files_from_paths(paths);
        }
    }

    /// Drop the loaded image and its textures so a record can never be analysed against, or shown
    /// with, an image that belongs to another patient or examination.
    fn clear_loaded_image(&mut self) {
        for texture in &mut self.mode_textures {
            *texture = None;
        }
        self.uploaded_filename = None;
        self.uploaded_dimensions = None;
        self.exam_view.image_path = None;
        self.exam_view.dimensions = None;
    }

    /// A new chat or a different patient starts with no examination, no image and no eye: the eye is
    /// never carried over from a previously opened record, and a draft owner is not kept selected.
    fn forget_exam_context(&mut self) {
        self.clear_loaded_image();
        self.active_examination_id = None;
        self.exam_view.laterality = EyeLaterality::Unspecified;
        let keep = self.active_patient_id.filter(|id| {
            !self
                .patient_service
                .get_patient(*id)
                .is_ok_and(|p| p.patient_code == retinacare_persistence::UNASSIGNED_CODE)
        });
        self.active_patient_id = keep;
    }

    /// Load the stored eye and fundus image of a saved examination, when the service knows it.
    fn load_exam_media(&mut self, ctx: &egui::Context, patient_id: i64, exam_id: i64) {
        let Ok(exams) = self.patient_service.list_examinations(patient_id) else {
            return;
        };
        let Some(ex) = exams.iter().find(|e| e.id == exam_id) else {
            return;
        };
        self.exam_view.laterality = EyeLaterality::from_db(&ex.eye);
        if !ex.image_path.is_empty() {
            let p = std::path::PathBuf::from(&ex.image_path);
            if p.exists() {
                self.load_fundus_from_path(ctx, &p);
            }
        }
    }

    /// Cancel and forget any cascade run and drop every result texture. Called whenever the patient,
    /// examination or image changes so nothing computed for another record can stay visible.
    fn reset_analysis_context(&mut self) {
        if let Some(id) = self.cascade_ctl.abandon() {
            self.backend_service.cancel_cascade(id);
            self.mark_card_abandoned(id);
        }
        self.exam_view.cascade.clear_run();
        self.exam_view.cascade.finishing_previous = self.cascade_ctl.finishing_previous();
        self.exam_view.lesion_artifact.clear();
        self.exam_view.vessel_artifact.clear();
        self.stage1_result = None;
        self.stage1_error = None;
        self.active_screening_request = None;
    }

    fn refresh_cascade_availability(&mut self) {
        match self.backend_service.cascade_status() {
            Ok(()) => {
                self.exam_view.cascade.available = true;
                self.exam_view.cascade.unavailable_reason = None;
            }
            Err(reason) => {
                self.exam_view.cascade.available = false;
                self.exam_view.cascade.unavailable_reason = Some(reason);
            }
        }
        self.exam_view.cascade.finishing_previous = self.cascade_ctl.finishing_previous();
    }

    /// Analyze action: the real cascade when the backend has all seven models, otherwise the
    /// Stage I-only path (kept for mock services and incomplete installs, and labelled as such).
    fn start_analysis(&mut self) {
        crate::ui2::console::log("analysis: start requested");
        use crate::engine::cascade::SCRATCH_EXAM_BASE;
        if self.backend_service.cascade_status().is_err() {
            self.start_stage1_only();
            return;
        }
        // The chat model must not hold VRAM the retinal models need (8 GB cards).
        crate::engine::ollama::release_vram();
        self.scratch_seq += 1;
        self.unsaved_run = None;
        let exam_key = self
            .auto_exam_for_run()
            .unwrap_or(SCRATCH_EXAM_BASE + self.scratch_seq);
        let request = self.cascade_ctl.prepare(
            self.active_patient_id,
            exam_key,
            self.exam_view.laterality,
            self.exam_view.image_path.as_deref(),
            self.exam_view.dimensions,
        );
        self.exam_view.cascade.error = None;
        let request = match request {
            Ok(request) => request,
            Err(refusal) => {
                self.exam_view.cascade.error = Some(refusal.to_string());
                return;
            }
        };
        let id = request.request_id;
        match self.backend_service.submit_cascade(request) {
            Ok(()) => {
                self.stage1_result = None;
                self.stage1_error = None;
                self.exam_view.lesion_artifact.clear();
                self.exam_view.vessel_artifact.clear();
                self.exam_view.cascade.begin(id);
                self.gpu_sampler = Some(crate::engine::gpu::GpuSampler::start());
            }
            Err(error) => {
                self.cascade_ctl.rollback(id);
                self.exam_view.cascade.error = Some(error.to_string());
            }
        }
    }

    /// The saved examination this run belongs to: the active one when it holds the same image, else a
    /// new one for the active patient. None (unsaved scratch run) without a patient, a confirmed eye
    /// or a readable image: laterality is never guessed.
    fn auto_exam_for_run(&mut self) -> Option<i64> {
        let path = self.exam_view.image_path.clone()?;
        // No patient chosen: the run becomes a draft owned by the detail-free "Unassigned" record.
        let patient_id = match self.active_patient_id {
            Some(id) => id,
            None => {
                let id = self.patient_service.ensure_unassigned_patient().ok()?;
                self.draft_owner = Some(id);
                self.active_patient_id = Some(id);
                id
            }
        };
        let eye = self.exam_view.laterality.db_code();
        let hash = crate::security::compute_sha256(&std::fs::read(&path).ok()?);
        let exams = self.patient_service.list_examinations(patient_id).ok()?;
        // The active exam is reused only when its stored image is readable and the same picture. An
        // exam of another patient's draft is never reused for this patient.
        if let Some(id) = self.active_examination_id {
            match exams.iter().find(|e| e.id == id) {
                Some(e) => {
                    // An exam with no image yet takes this one. A stored image that is unreadable or
                    // different is never overwritten: a new exam is made instead.
                    let same = e.image_path.is_empty()
                        || std::fs::read(&e.image_path)
                            .is_ok_and(|b| crate::security::compute_sha256(&b) == hash);
                    if same {
                        return Some(id);
                    }
                }
                None => {
                    let other_draft = self.draft_owner.is_some_and(|d| {
                        self.patient_service
                            .list_examinations(d)
                            .is_ok_and(|l| l.iter().any(|e| e.id == id))
                    });
                    if !other_draft {
                        return Some(id);
                    }
                }
            }
        }
        let patient = self.patient_service.get_patient(patient_id).ok()?;
        let id = self
            .patient_service
            .add_examination(&crate::engine::ExaminationSummary {
                id: 0,
                patient_id,
                patient_code: patient.patient_code.clone(),
                examined_at: crate::engine::report::utc_now(),
                operator: current_operator(),
                notes: String::new(),
                eye: eye.to_string(),
                image_path: crate::engine::store::keep_exam_image(&path)
                    .to_string_lossy()
                    .to_string(),
                has_image: true,
                has_analysis: false,
                analysis_summary: None,
                is_synthetic: patient.is_synthetic,
            })
            .ok()?;
        self.active_examination_id = Some(id);
        self.exam_view.patient_code = patient.patient_code;
        self.exam_view.exam_id = format!("EXAM-{id:03}");
        Some(id)
    }

    fn start_stage1_only(&mut self) {
        if let Some(path) = &self.exam_view.image_path {
            let req = crate::engine::RetinalAnalysisRequest::new(
                self.exam_view.exam_id.clone(),
                path.clone(),
                Some(self.exam_view.laterality),
            );
            self.active_screening_request = Some(req.clone());
            self.stage1_error = None;
            if let Err(err) = self.backend_service.submit(req) {
                self.stage1_error = Some(format!("Submission failed: {err}"));
                self.active_screening_request = None;
            } else {
                self.backend_readiness = self.backend_service.readiness();
            }
        }
    }

    fn cancel_analysis(&mut self) {
        crate::ui2::console::log("analysis: cancel requested");
        if let Some(job) = self.cascade_ctl.active() {
            if self.backend_service.cancel_cascade(job.request_id) {
                self.exam_view.cascade.phase = crate::engine::cascade::CascadePhase::Cancelling;
            }
        } else if let Some(req) = &self.active_screening_request {
            self.backend_service.cancel(&req.request_id);
            self.backend_readiness = self.backend_service.readiness();
        }
    }

    /// Bounded per-frame drain of the cascade channel; never blocks.
    fn drain_cascade_events(&mut self, ctx: &egui::Context) {
        use crate::engine::cascade::{AnalysisEvent, Routed};
        for _ in 0..64 {
            let Some(event) = self.backend_service.try_recv_cascade() else {
                break;
            };
            match self.cascade_ctl.route(&event) {
                Routed::Stale => {}
                Routed::Progress => self.exam_view.cascade.apply_progress(&event),
                Routed::FinishedAbandoned => {}
                Routed::FinishedActive(job) => {
                    if let AnalysisEvent::Finished(report) = event {
                        self.finish_cascade(ctx, &job, &report);
                    }
                }
            }
            ctx.request_repaint();
        }
        self.exam_view.cascade.finishing_previous = self.cascade_ctl.finishing_previous();
    }

    /// Put one mask on the matching Quad Viewer panel (lesion panel or vessel panel).
    fn install_mask_texture(
        &mut self,
        ctx: &egui::Context,
        name: String,
        kind: retinacare_ai_bridge::stage2_contract::Stage2ModelKind,
        model_version: &str,
        image: egui::ColorImage,
        foreground_pixels: u64,
    ) {
        use retinacare_ai_bridge::stage2_contract::Stage2ModelKind;
        let texture = crate::domain::MaskTexture {
            texture: ctx.load_texture(name, image, egui::TextureOptions::NEAREST),
            foreground_pixels,
        };
        match kind {
            Stage2ModelKind::Vessel => {
                let vessels = &mut self.exam_view.vessel_artifact;
                vessels.texture = Some(texture);
                vessels.model_version = model_version.to_string();
                vessels.status = crate::domain::ArtifactLoadStatus::ArtifactReady;
            }
            other => {
                let lesions = &mut self.exam_view.lesion_artifact;
                match other {
                    Stage2ModelKind::Ma => lesions.textures.ma = Some(texture),
                    Stage2ModelKind::He => lesions.textures.he = Some(texture),
                    Stage2ModelKind::Ex => lesions.textures.ex = Some(texture),
                    _ => lesions.textures.cws = Some(texture),
                }
                lesions.model_version = model_version.to_string();
                lesions.status = crate::domain::ArtifactLoadStatus::ArtifactReady;
            }
        }
    }

    fn finish_cascade(
        &mut self,
        ctx: &egui::Context,
        job: &crate::engine::cascade::JobIdentity,
        report: &crate::engine::cascade::AnalysisReport,
    ) {
        use crate::engine::cascade::{self, OutcomeState};
        let identity = cascade::verify_identity(job, report);
        let verified = identity.is_ok();
        let mut outcome = cascade::outcome_from_report(report, identity);
        outcome.gpu = self.gpu_sampler.take().and_then(|s| s.finish());
        // Show pixels and the Stage I card only for a verified, completed run.
        let completed = verified
            && matches!(
                outcome.state,
                OutcomeState::Graded | OutcomeState::Ungradable | OutcomeState::NegativeRoute
            );
        self.exam_view.lesion_artifact.clear();
        self.exam_view.vessel_artifact.clear();
        if completed {
            for mask in &report.masks {
                let Some(image) = cascade::mask_image(mask) else {
                    continue;
                };
                let foreground = outcome
                    .masks
                    .iter()
                    .find(|m| {
                        m.key
                            == cascade::StageKey::from_stage(cascade::AnalysisStage::Segmentation(
                                mask.model_kind,
                            ))
                    })
                    .map_or(0, |m| m.foreground_pixels);
                self.install_mask_texture(
                    ctx,
                    format!("mask_{:?}_{}", mask.model_kind, job.request_id),
                    mask.model_kind,
                    &mask.model_version,
                    image,
                    foreground,
                );
            }
            if let Some(gate) = &report.screening {
                self.stage1_result = Some(crate::engine::Stage1ScreeningResult {
                    request_id: format!("cascade-{}", report.request_id),
                    examination_id: self.exam_view.exam_id.clone(),
                    model_id: gate.model_id.clone(),
                    model_version: gate.version.clone(),
                    model_sha256: gate.sha256.clone(),
                    provider: match gate.provider {
                        retinacare_ai_bridge::device::ActualProvider::Cpu => "CPU Execution Provider".to_string(),
                        retinacare_ai_bridge::device::ActualProvider::Cuda { device_id } => format!("CUDA device {device_id}"),
                    },
                    raw_logit: gate.gate.raw_logit,
                    screening_score: gate.gate.screening_score,
                    threshold: gate.gate.threshold,
                    decision: gate.gate.decision,
                    elapsed_ms: (gate.elapsed_microseconds / 1000) as u64,
                    preprocessing_ms: (gate.preprocessing_microseconds.unwrap_or(0) / 1000) as u64,
                    disclaimer: "Research and screening triage prototype only. Not certified for clinical diagnosis. Clinical examination required for definitive care.".to_string(),
                });
            }
        }
        self.exam_view.cascade.finish(outcome);
        self.after_cascade_finished(job, report);
        self.bilateral_on_finished(job);
    }

    /// Store the five masks of a completed run with the saved examination and describe the outcome.
    /// A negative screening route has no masks, which is not an error.
    fn save_masks_note(&mut self, report: &crate::engine::cascade::AnalysisReport) -> String {
        const SUMMARY: &str = "Result summary saved with the examination (encrypted database).";
        if report.masks.is_empty() {
            self.exam_view.cascade.mask_note = None;
            return format!("{SUMMARY} No masks were produced for this route.");
        }
        let stored = crate::engine::cascade::mask_run_from_report(report)
            .map_err(|e| e.to_string())
            .and_then(|run| {
                self.patient_service
                    .save_examination_masks(run)
                    .map_err(|e| e.to_string())
            });
        match stored {
            Ok(()) => {
                self.exam_view.cascade.mask_note = None;
                format!("{SUMMARY} Five masks stored (checksummed) with it.")
            }
            Err(e) => {
                self.exam_view.cascade.mask_note = Some(format!("Masks NOT stored: {e}"));
                format!("{SUMMARY} Masks NOT stored: {e}")
            }
        }
    }

    /// Store the summary, image reference and masks of a completed run with its saved examination.
    fn persist_completed_run(
        &mut self,
        job: &crate::engine::cascade::JobIdentity,
        report: &crate::engine::cascade::AnalysisReport,
        outcome: &crate::engine::cascade::CascadeOutcome,
    ) -> String {
        let Ok(json) = serde_json::to_string(outcome) else {
            return "Result NOT saved: could not serialize the result.".to_string();
        };
        let eye = self.exam_view.laterality.db_code();
        let image = crate::engine::store::keep_exam_image(&job.image_path)
            .to_string_lossy()
            .to_string();
        let saved = self
            .patient_service
            .set_examination_image(job.examination_id, eye, &image)
            .and_then(|()| {
                self.patient_service
                    .save_examination_analysis(job.examination_id, &json)
            });
        match saved {
            Ok(()) => {
                // A new run replaces the masks and grade: earlier explanations no longer describe it.
                self.patient_service
                    .mark_attributions_stale(job.examination_id);
                self.explain.history_loaded_for = None;
                self.prune_orphan_exam_images();
                self.save_masks_note(report)
            }
            Err(e) => format!("Result shown but NOT saved: {e}"),
        }
    }

    /// Remove copied images that no saved examination points at (self-healing after exam changes).
    fn prune_orphan_exam_images(&self) {
        let Ok(patients) = self
            .patient_service
            .list_patients(&crate::engine::PatientQuery::default())
        else {
            return;
        };
        let mut referenced = std::collections::HashSet::new();
        for p in patients {
            let Ok(exams) = self.patient_service.list_examinations(p.id) else {
                return; // an unreadable list must never turn into deleting everything
            };
            referenced.extend(
                exams
                    .into_iter()
                    .map(|e| std::path::PathBuf::from(e.image_path)),
            );
        }
        crate::engine::store::prune_exam_images(
            &crate::engine::store::exam_images_dir(),
            &referenced,
        );
    }

    fn open_save_dialog(&mut self) {
        if self.unsaved_run.is_none() {
            return;
        }
        let patients = self
            .patient_service
            .list_patients(&crate::engine::PatientQuery::default())
            .unwrap_or_default();
        let patient_id = self
            .active_patient_id
            .filter(|id| patients.iter().any(|p| p.id == *id));
        let exams = patient_id
            .and_then(|id| self.patient_service.list_examinations(id).ok())
            .unwrap_or_default();
        let hint = self
            .uploaded_filename
            .as_deref()
            .and_then(EyeLaterality::hint_from_filename);
        self.save_dialog = Some(ui::save_exam::SaveDialog {
            patients,
            patient_id,
            exams,
            target: ui::save_exam::SaveTarget::New,
            eye: self.exam_view.laterality.db_code().to_string(),
            hint: hint.map(|h| h.db_code().to_string()),
            acknowledged: false,
            error: None,
            assign_exam: None,
        });
    }

    /// Explicit user action: attach the unsaved run to a chosen (or new) examination of a chosen patient.
    fn save_unsaved_run(
        &mut self,
        patient_id: i64,
        target: ui::save_exam::SaveTarget,
    ) -> Result<(), String> {
        let (mut job, mut report) = self
            .unsaved_run
            .clone()
            .ok_or("no unsaved analysis is on screen")?;
        let mut outcome = self
            .exam_view
            .cascade
            .outcome
            .clone()
            .filter(|o| o.request_id == job.request_id && o.identity_verified)
            .ok_or("the analysis on screen is not the verified run")?;
        let patient = self
            .patient_service
            .get_patient(patient_id)
            .map_err(|e| e.to_string())?;
        let exams = self
            .patient_service
            .list_examinations(patient_id)
            .map_err(|e| e.to_string())?;
        let exam_id = match target {
            ui::save_exam::SaveTarget::Existing(id) => {
                let Some(ex) = exams.iter().find(|e| e.id == id) else {
                    return Err("that examination does not belong to the chosen patient".into());
                };
                let run_eye = self.exam_view.laterality.db_code();
                if ex.eye != run_eye && matches!(ex.eye.as_str(), "OD" | "OS") {
                    return Err(format!(
                        "this examination is recorded as {} but the analysis is for {run_eye}",
                        ex.eye
                    ));
                }
                id
            }
            ui::save_exam::SaveTarget::New => {
                let eye = self.exam_view.laterality.db_code();
                self.patient_service
                    .add_examination(&crate::engine::ExaminationSummary {
                        id: 0,
                        patient_id,
                        patient_code: patient.patient_code.clone(),
                        examined_at: crate::engine::report::utc_now(),
                        operator: current_operator(),
                        notes: String::new(),
                        eye: eye.to_string(),
                        image_path: job.image_path.to_string_lossy().to_string(),
                        has_image: true,
                        has_analysis: false,
                        analysis_summary: None,
                        is_synthetic: patient.is_synthetic,
                    })
                    .map_err(|e| format!("could not create the examination: {e}"))?
            }
        };
        job.examination_id = exam_id;
        job.patient_id = Some(patient_id);
        report.examination_id = exam_id;
        report.patient_id = Some(patient_id);
        outcome.examination_id = exam_id;
        outcome.patient_id = Some(patient_id);
        let note = self.persist_completed_run(&job, &report, &outcome);
        if !note.contains("saved with the examination") {
            return Err(note);
        }
        self.exam_view.cascade.outcome = Some(outcome);
        self.exam_view.cascade.save_note = Some(note);
        self.unsaved_run = None;
        self.active_patient_id = Some(patient_id);
        self.active_examination_id = Some(exam_id);
        self.exam_view.patient_code = patient.patient_code;
        self.exam_view.exam_id = format!("EXAM-{exam_id:03}");
        self.finish_analysis_card(job.request_id);
        let _ = self
            .audit_ledger_service
            .record_event(crate::engine::AuditEvent {
                event_id: String::new(),
                timestamp: crate::engine::report::utc_now(),
                event_type: crate::engine::AuditEventType::AnalysisCompleted,
                examination_id: self.exam_view.exam_id.clone(),
                request_id: format!("cascade-{}", job.request_id),
                operator: current_operator(),
                status: crate::engine::AuditEventStatus::Success,
                non_sensitive_summary: "Unsaved analysis saved to the examination by the user"
                    .to_string(),
            });
        Ok(())
    }

    /// Explicit user action step 1: offer the real patients for a draft examination.
    fn open_assign_dialog(&mut self, exam_id: i64) {
        let patients = self
            .patient_service
            .list_patients(&crate::engine::PatientQuery::default())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.patient_code != retinacare_persistence::UNASSIGNED_CODE)
            .collect();
        self.save_dialog = Some(ui::save_exam::SaveDialog {
            patients,
            patient_id: None,
            exams: Vec::new(),
            target: ui::save_exam::SaveTarget::New,
            eye: self.exam_view.laterality.db_code().to_string(),
            hint: None,
            acknowledged: false,
            error: None,
            assign_exam: Some(exam_id),
        });
    }

    /// Explicit user action step 2: move a draft examination, with its masks and explanations, to
    /// the confirmed patient. Refuses anything that is not a draft.
    fn assign_draft(&mut self, exam_id: i64, patient_id: i64) -> Result<(), String> {
        let patient = self
            .patient_service
            .get_patient(patient_id)
            .map_err(|e| e.to_string())?;
        self.patient_service
            .assign_examination(exam_id, patient_id)
            .map_err(|e| format!("could not assign the draft: {e}"))?;
        self.active_patient_id = Some(patient_id);
        self.active_examination_id = Some(exam_id);
        self.exam_view.patient_code = patient.patient_code.clone();
        if let Some(o) = self.exam_view.cascade.outcome.as_mut() {
            if o.examination_id == exam_id {
                o.patient_id = Some(patient_id);
            }
        }
        let mut touched = Vec::new();
        for (i, m) in self.chat_messages.iter_mut().enumerate() {
            if let Some(c) = m.card.as_mut().filter(|c| c.examination_id == exam_id) {
                c.patient_id = Some(patient_id);
                touched.push(i);
            }
        }
        for i in touched {
            self.update_stored(i);
        }
        self.sync_session_snapshot();
        // The old heatmap was stale-marked for the old owner; make one for the new owner.
        self.explain.status = retinacare_ai_bridge::attribution::AttributionStatus::NotGenerated;
        self.explain.display = None;
        self.explain_auto_start();
        let _ = self
            .audit_ledger_service
            .record_event(crate::engine::AuditEvent {
                event_id: String::new(),
                timestamp: crate::engine::report::utc_now(),
                event_type: crate::engine::AuditEventType::AnalysisCompleted,
                examination_id: format!("EXAM-{exam_id:03}"),
                request_id: String::new(),
                operator: current_operator(),
                status: crate::engine::AuditEventStatus::Success,
                non_sensitive_summary: "Draft examination assigned to a patient by the user"
                    .to_string(),
            });
        Ok(())
    }

    fn save_dialog_ui(&mut self, ctx: &egui::Context) {
        use ui::save_exam::SaveAction;
        let Some(mut dialog) = self.save_dialog.take() else {
            return;
        };
        match ui::save_exam::render(ctx, &mut dialog) {
            SaveAction::None => self.save_dialog = Some(dialog),
            SaveAction::Cancel => {}
            SaveAction::PatientChanged(id) => {
                dialog.exams = self
                    .patient_service
                    .list_examinations(id)
                    .unwrap_or_default();
                dialog.target = ui::save_exam::SaveTarget::New;
                dialog.error = None;
                self.save_dialog = Some(dialog);
            }
            SaveAction::Confirm => {
                let result = dialog
                    .patient_id
                    .ok_or_else(|| "choose a patient".to_string())
                    .and_then(|id| match dialog.assign_exam {
                        Some(exam) => self.assign_draft(exam, id),
                        None => self.save_unsaved_run(id, dialog.target),
                    });
                if let Err(e) = result {
                    dialog.error = Some(format!("Not saved: {e}"));
                    self.save_dialog = Some(dialog);
                }
            }
        }
    }

    /// Persist (when the examination is saved), audit, and note the outcome of a finished run.
    fn after_cascade_finished(
        &mut self,
        job: &crate::engine::cascade::JobIdentity,
        report: &crate::engine::cascade::AnalysisReport,
    ) {
        use crate::engine::cascade::{OutcomeState, SCRATCH_EXAM_BASE};
        use crate::engine::{AuditEventStatus, AuditEventType};
        let Some(outcome) = self.exam_view.cascade.outcome.clone() else {
            return;
        };
        let saved_exam = job.examination_id < SCRATCH_EXAM_BASE;
        let completed = outcome.identity_verified
            && matches!(
                outcome.state,
                OutcomeState::Graded | OutcomeState::Ungradable | OutcomeState::NegativeRoute
            );
        self.exam_view.cascade.save_note = Some(if !completed {
            "Not saved: only a verified, completed result is stored.".to_string()
        } else if !saved_exam {
            "Not saved: this image is not attached to a saved examination. Create an examination for the patient to keep results.".to_string()
        } else {
            self.persist_completed_run(job, report, &outcome)
        });
        self.unsaved_run = (completed && !saved_exam).then(|| (job.clone(), report.clone()));
        // Once saved, read the picture from the app's own copy: the upload may be moved or deleted.
        if completed && saved_exam {
            let kept = self.active_patient_id.and_then(|p| {
                self.patient_service
                    .list_examinations(p)
                    .ok()?
                    .into_iter()
                    .find(|e| e.id == job.examination_id)
                    .map(|e| std::path::PathBuf::from(e.image_path))
            });
            if let Some(kept) = kept.filter(|k| k.is_file()) {
                self.exam_view.image_path = Some(kept);
            }
        }
        if completed && saved_exam && outcome.state == OutcomeState::Graded {
            self.explain_auto_start();
        }
        let (kind, status) = match outcome.state {
            OutcomeState::Cancelled => (
                AuditEventType::AnalysisCancelled,
                AuditEventStatus::Cancelled,
            ),
            OutcomeState::Failed => (AuditEventType::AnalysisFailed, AuditEventStatus::Failed),
            _ => (AuditEventType::AnalysisCompleted, AuditEventStatus::Success),
        };
        let label = if saved_exam {
            format!("EXAM-{:03}", job.examination_id)
        } else {
            self.exam_view.exam_id.clone()
        };
        let _ = self
            .audit_ledger_service
            .record_event(crate::engine::AuditEvent {
                event_id: String::new(),
                timestamp: crate::engine::report::utc_now(),
                event_type: kind,
                examination_id: label,
                request_id: format!("cascade-{}", outcome.request_id),
                operator: current_operator(),
                status,
                non_sensitive_summary: format!(
                    "{} (grade {}, {:.1} s, provider {})",
                    outcome.state.label(),
                    outcome.grade.map_or("none".to_string(), |g| g.to_string()),
                    outcome.total_seconds,
                    outcome
                        .screening
                        .as_ref()
                        .map_or("n/a", |s| s.provider.as_str()),
                ),
            });
        self.cached_audit_events = self
            .audit_ledger_service
            .query_events(Some(&self.exam_view.exam_id))
            .unwrap_or_default();
        self.finish_analysis_card(job.request_id);
    }

    /// Show the stored summary and the verified stored masks of a saved examination. Reading history
    /// never runs the models.
    fn restore_saved_analysis(&mut self, ctx: &egui::Context, examination_id: i64) {
        self.exam_view.cascade.outcome = None;
        self.exam_view.cascade.restored = false;
        self.exam_view.cascade.masks_restored = false;
        self.exam_view.cascade.mask_note = None;
        if let Some(json) = self.patient_service.load_analysis(examination_id) {
            if let Ok(outcome) =
                serde_json::from_str::<crate::engine::cascade::CascadeOutcome>(&json)
            {
                if outcome.examination_id == examination_id {
                    // The Stage I card and route pill read this; it is rebuilt from the stored
                    // summary only, so the fields the summary does not hold stay marked as such.
                    self.stage1_result =
                        outcome
                            .screening
                            .as_ref()
                            .map(|s| crate::engine::Stage1ScreeningResult {
                                request_id: format!("stored-{}", outcome.request_id),
                                examination_id: format!("EXAM-{examination_id:03}"),
                                model_id: s.model_id.clone(),
                                model_version: if s.model_version.is_empty() {
                                    "not recorded".to_string()
                                } else {
                                    s.model_version.clone()
                                },
                                model_sha256: s.model_sha256.clone(),
                                provider: format!("{} (stored summary)", s.provider),
                                raw_logit: s.raw_logit,
                                screening_score: s.score,
                                threshold: s.threshold,
                                decision: if s.continue_cascade {
                                    crate::engine::Stage1GateDecision::ContinueCascade
                                } else {
                                    crate::engine::Stage1GateDecision::NegativeRoute
                                },
                                elapsed_ms: s.elapsed_ms,
                                preprocessing_ms: 0,
                                disclaimer: crate::engine::MANDATORY_RESEARCH_CAVEAT.to_string(),
                            });
                    self.exam_view.cascade.restored = true;
                    self.restore_stored_masks(ctx, examination_id, &outcome);
                    self.exam_view.cascade.outcome = Some(outcome);
                }
            }
        }
    }

    /// Load the five masks of a saved examination after checking patient, eye, image hash and the
    /// run that produced the stored summary. Any failure leaves the panels empty and says why.
    fn restore_stored_masks(
        &mut self,
        ctx: &egui::Context,
        examination_id: i64,
        outcome: &crate::engine::cascade::CascadeOutcome,
    ) {
        use crate::engine::cascade;
        if outcome.masks.is_empty() {
            return;
        }
        let (Some(patient_id), Some(eye)) = (
            self.active_patient_id,
            Some(outcome.eye.as_deref().unwrap_or("Unknown")),
        ) else {
            self.exam_view.cascade.mask_note =
                Some("Stored masks not loaded: the patient is unknown.".to_string());
            return;
        };
        let run = match self.patient_service.load_examination_masks(
            examination_id,
            patient_id,
            eye,
            outcome.image_sha256.as_deref(),
        ) {
            Ok(Some(run)) => run,
            Ok(None) => {
                self.exam_view.cascade.mask_note = Some(
                    "No stored masks for this examination (saved before mask storage). Run the analysis again to view them."
                        .to_string(),
                );
                return;
            }
            Err(e) => {
                self.exam_view.cascade.mask_note = Some(format!(
                    "Stored masks not shown: {e}. Run the analysis again."
                ));
                return;
            }
        };
        if run.input.request_id != outcome.request_id
            || run.input.image_width != outcome.image_width.unwrap_or(0)
            || run.input.image_height != outcome.image_height.unwrap_or(0)
        {
            self.exam_view.cascade.mask_note = Some(
                "Stored masks do not belong to the stored summary; not shown. Run the analysis again."
                    .to_string(),
            );
            return;
        }
        let mut images = Vec::new();
        for mask in &run.input.masks {
            let image = cascade::kind_from_label(&mask.kind).and_then(|kind| {
                cascade::mask_image_from_pixels(
                    kind,
                    mask.width as usize,
                    mask.height as usize,
                    &mask.pixels,
                )
                .map(|image| (kind, image))
            });
            let Some((kind, image)) = image else {
                self.exam_view.cascade.mask_note =
                    Some("Stored masks are malformed; not shown.".to_string());
                return;
            };
            let foreground = mask.pixels.iter().filter(|p| **p == 255).count() as u64;
            images.push((kind, mask.model_version.clone(), image, foreground));
        }
        for (kind, version, image, foreground) in images {
            self.install_mask_texture(
                ctx,
                format!("stored_mask_{kind:?}_{examination_id}"),
                kind,
                &version,
                image,
                foreground,
            );
        }
        self.exam_view.cascade.masks_restored = true;
    }

    /// Build the printed report from this examination's verified results only.
    fn build_pdf_document(&self) -> retinacare_reports::PdfDocument {
        ui2::fonts::install_pdf_fonts();
        let patient = self
            .active_patient_id
            .and_then(|id| self.patient_service.get_patient(id).ok());
        let examined_at = self
            .active_patient_id
            .zip(self.active_examination_id)
            .and_then(|(pid, eid)| {
                self.patient_service
                    .list_examinations(pid)
                    .ok()?
                    .into_iter()
                    .find(|e| e.id == eid)
                    .map(|e| e.examined_at)
            });
        let eye = match self.exam_view.laterality {
            EyeLaterality::OD => "OD (right)",
            EyeLaterality::OS => "OS (left)",
            EyeLaterality::Unspecified => "Unknown (not confirmed)",
        };
        crate::engine::report::build_report_document(&crate::engine::report::ReportContext {
            patient_code: &self.exam_view.patient_code,
            patient_name: patient.as_ref().map(|p| p.display_name.as_str()),
            exam_label: &self.exam_view.exam_id,
            eye,
            examined_at: examined_at.as_deref(),
            image_name: self.uploaded_filename.as_deref(),
            image_dims: self.uploaded_dimensions,
            generated_utc: crate::engine::report::utc_now(),
            stage1: self.stage1_result.as_ref(),
            outcome: self.exam_view.cascade.outcome.as_ref(),
            explanation: self.explain_section().as_ref(),
            evidence: self.report_evidence().as_ref(),
        })
    }

    /// Plugin cards for the new design; the status shown is the app's real state.
    fn plugin_cards(&self) -> Vec<crate::ui2::plugins_page::Card> {
        use crate::ui2::{
            icon_paths as ic,
            plugins_page::{Badge, Card},
        };
        let planned = |icon, title, detail| Card {
            icon,
            title,
            badge: Badge::Planned,
            badge_text: "Planned".to_owned(),
            detail,
        };
        let (assist_badge, assist_text) = match &self.composer.assistant_status {
            Some(Ok(_)) => (Badge::Ready, "Ready"),
            Some(Err(_)) => (Badge::Off, "Not running"),
            None => (Badge::Off, "Checking"),
        };
        let (cloud_badge, cloud_text) = if self.composer.cloud_available {
            (Badge::Ready, "Signed in")
        } else {
            (Badge::Off, "Signed out")
        };
        vec![
            planned(
                ic::P_DECK,
                "Report deck",
                "Build a slide deck from a finished exam.",
            ),
            planned(
                ic::P_MCP,
                "MCP pipeline",
                "Run connected tools on verified results.",
            ),
            Card {
                icon: ic::P_ASSIST,
                title: "Local assistant",
                badge: assist_badge,
                badge_text: assist_text.to_owned(),
                detail: "Ollama on this computer. Loopback only.",
            },
            Card {
                icon: ic::P_CLOUD,
                title: "Cloud sync",
                badge: cloud_badge,
                badge_text: cloud_text.to_owned(),
                detail: "Optional encrypted chat sync. Never images.",
            },
            planned(
                ic::P_EXPORT,
                "Research export",
                "CSV and PNG exports for studies.",
            ),
            planned(
                ic::P_AUTO,
                "Automation",
                "Scripted runs for batches of images.",
            ),
        ]
    }

    /// Asks where to save, then starts the background PDF export of the current report.
    fn submit_pdf_export(&mut self) {
        match self.choose_pdf_destination() {
            None => {
                self.export_state = crate::engine::ExportState::Cancelled {
                    request_id: "no_destination".to_string(),
                };
            }
            Some(dest) => {
                let request_id = format!(
                    "pdf_req_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                );
                let req = crate::engine::ReportExportRequest {
                    request_id,
                    examination_id: self.exam_view.exam_id.clone(),
                    patient_code: self.exam_view.patient_code.clone(),
                    destination_path: dest,
                    include_original_image: false,
                    include_findings_json: false,
                    include_provenance_meta: true,
                    // The exported file is the report the user is looking at, not a second build.
                    document: Some(
                        self.report_doc
                            .clone()
                            .unwrap_or_else(|| self.build_pdf_document()),
                    ),
                };
                match self.pdf_export_service.submit_export(req) {
                    Ok(rx) => {
                        self.export_event_rx = Some(rx);
                        self.export_state = crate::engine::ExportState::Preparing;
                    }
                    Err(err) => {
                        self.export_state = crate::engine::ExportState::Failed {
                            request_id: "req_failed".to_string(),
                            error: err,
                        };
                    }
                }
            }
        }
    }

    /// Where to save the PDF: `RETINACARE_PDF_DEST` (automation), else the user's save dialog.
    fn choose_pdf_destination(&self) -> Option<std::path::PathBuf> {
        if let Some(path) = std::env::var_os("RETINACARE_PDF_DEST") {
            return Some(std::path::PathBuf::from(path));
        }
        rfd::FileDialog::new()
            .set_file_name(format!(
                "retinacare_{}_{}.pdf",
                self.exam_view.patient_code, self.exam_view.exam_id
            ))
            .add_filter("PDF report", &["pdf"])
            .save_file()
    }

    /// Loads a fundus image from a file path into Box 0 ("Original Image")
    fn load_fundus_from_path(&mut self, ctx: &egui::Context, path: &std::path::Path) {
        /// Same decoded-pixel ceiling the backend enforces (`MAX_IMAGE_PIXELS`).
        const MAX_DECODED_PIXELS: u64 = 40_000_000;
        let valid_path = match validate_attachment_path(path) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("Image load blocked by security policy: {err}");
                self.exam_view.cascade.error = Some(format!("Image rejected: {err}"));
                return;
            }
        };
        let bytes = match std::fs::read(&valid_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.exam_view.cascade.error =
                    Some(format!("The image file cannot be read: {err}"));
                return;
            }
        };
        let clean_bytes = scrub_image_metadata(&bytes);
        // Refuse before decoding: a small file can declare billions of pixels.
        let declared = image::ImageReader::new(std::io::Cursor::new(&clean_bytes))
            .with_guessed_format()
            .ok()
            .and_then(|reader| reader.into_dimensions().ok());
        match declared {
            Some((w, h)) if u64::from(w) * u64::from(h) > MAX_DECODED_PIXELS => {
                self.exam_view.cascade.error = Some(format!(
                    "Image rejected: {w}x{h} pixels exceeds the {MAX_DECODED_PIXELS}-pixel limit."
                ));
                return;
            }
            None => {
                self.exam_view.cascade.error = Some(
                    "Image rejected: the file is not a readable image (corrupt or unsupported format)."
                        .to_string(),
                );
                return;
            }
            _ => {}
        }
        let Ok(image) = image::load_from_memory(&clean_bytes) else {
            self.exam_view.cascade.error = Some(
                "Image rejected: the file is not a readable image (corrupt or unsupported format)."
                    .to_string(),
            );
            return;
        };
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        self.uploaded_dimensions = Some(size);
        self.uploaded_filename = path
            .file_name()
            .map(|n| crate::ui::explain::ellipsize(&n.to_string_lossy(), 40));

        let img0 = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
        self.mode_textures[0] =
            Some(ctx.load_texture("mode_orig", img0, egui::TextureOptions::LINEAR));

        self.mode_textures[1] = None;
        self.mode_textures[2] = None;
        self.mode_textures[3] = None;
        self.active_mode = 0;
        self.viewer.reset();
        self.exam_view.dimensions = Some(size);
        self.exam_view.image_path = Some(valid_path);
        self.reset_analysis_context();
        for state in &mut self.exam_view.panel_states {
            state.reset();
        }
    }

    /// User uploads ONLY ONE image, updating ONLY Box 0 ("Original Image").
    /// Boxes 1, 2, and 3 remain completely empty.
    fn pick_and_load_image(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Fundus Images", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
        {
            self.load_fundus_from_path(ctx, &path);
        }
    }
}

/// Bilateral Assessment seam. Every medical step is the existing single-eye cascade (`start_analysis`),
/// run for one eye at a time; this code only feeds it an eye's image and files each verified outcome
/// under that eye. Pure rules live in `ui2/bilateral_assessment/state.rs`.
impl RetinaCareApp {
    fn render_bilateral(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context) {
        use crate::ui2::bilateral_assessment::{self as ba, state::Eye, BilateralAction as A};
        let pal = crate::ui2::theme::mode(ctx).palette();
        let patient_label = match self.active_patient_id {
            Some(_) if !self.exam_view.patient_code.is_empty() => {
                format!("Patient {}", self.exam_view.patient_code)
            }
            Some(_) => "Patient selected".to_string(),
            None => "No patient selected (results become an unassigned draft)".to_string(),
        };
        let progress = self
            .exam_view
            .cascade
            .stages
            .iter()
            .find(|s| s.state == crate::engine::cascade::StageRunState::Running)
            .map(|s| format!("Running: {}", s.key.label()));
        let bx = ba::BilateralContext {
            patient_label: &patient_label,
            cascade_unavailable: self.exam_view.cascade.unavailable_reason.as_deref(),
            progress: progress.as_deref(),
        };
        let action = ba::show(ui, pal, rect, &mut self.bilateral, &bx);
        match action {
            A::None => {}
            A::Pick(eye) => self.bilateral_pick(eye),
            A::Run(eye) => self.bilateral_request(&[eye]),
            A::RunBoth => self.bilateral_request(&[Eye::Os, Eye::Od]),
            A::Cancel => {
                self.bilateral.session.cancel();
                self.cancel_analysis();
            }
            A::OpenImaging => self.active_page = AppPage::RetinalImaging,
            A::OpenSingleEye => self.active_page = AppPage::Report,
            A::ViewEye(eye) => self.bilateral_view_eye(ctx, eye),
        }
    }

    fn bilateral_pick(&mut self, eye: crate::ui2::bilateral_assessment::state::Eye) {
        use crate::ui2::bilateral_assessment::{intake, state::Eye};
        // Automation hook, like RETINACARE_AUTOPICK_IMAGE: one path per eye instead of the dialog.
        let hook = match eye {
            Eye::Os => "RETINACARE_BILATERAL_OS",
            Eye::Od => "RETINACARE_BILATERAL_OD",
        };
        let path = std::env::var_os(hook).map(std::path::PathBuf::from).or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Fundus Images", &["png", "jpg", "jpeg", "webp", "bmp"])
                .pick_file()
        });
        let Some(path) = path else { return };
        let loaded = match validate_attachment_path(&path) {
            Ok(valid) => intake::intake(&valid),
            Err(_) => Err(intake::IntakeError::Blocked),
        };
        self.bilateral.session.notice = self.bilateral.session.set_source(eye, loaded).err();
    }

    /// Chat image(s) wait here until the user states the eye side. Never guessed from the file or
    /// from the attachment order.
    fn eye_choice_window(&mut self, ctx: &egui::Context) {
        use crate::ui2::bilateral_assessment::{intake, state::{Eye, Mode}};
        enum Pick {
            Wait,
            One(usize, EyeLaterality),
            Both,
            Cancel,
        }
        let Some(c) = self.eye_choice.as_mut() else {
            self.composer.modal_open = false;
            return;
        };
        self.composer.modal_open = true;
        self.composer.attachment_menu_open = false;
        let mut pick = if c.patient != self.active_patient_id { Pick::Cancel } else { Pick::Wait };
        let imgs: Vec<String> = c
            .files
            .iter()
            .filter(|a| a.kind == AttachmentKind::FundusImage)
            .map(|a| a.display_name.clone())
            .collect();
        let eye_of = |i: usize, first_is_os: bool| {
            if (i == 0) == first_is_os { EyeLaterality::OS } else { EyeLaterality::OD }
        };
        use crate::ui2::{fonts, widgets::{self, ButtonKind}};
        let pal = crate::ui2::theme::mode(ctx).palette();
        // Close, then open: the dialog waits for the attachment menu's 0.16 s fade to finish.
        let waited = c.since.elapsed().as_secs_f32();
        if waited < 0.18 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(0.18 - waited));
            return;
        }
        // A real modal: it dims the page and blocks clicks behind it (the attach menu cannot stay open on top).
        let modal = egui::Modal::new(egui::Id::new("eye_choice"))
            .frame(widgets::card_frame(&pal))
            .show(ctx, |ui| {
                ui.set_width(440.0);
                ui.label(egui::RichText::new("Which eye are you uploading?").font(fonts::serif(24.0)).color(pal.text_primary));
                ui.add_space(12.0);
                let eye_name = |e: EyeLaterality| if e == EyeLaterality::OS { "OS - Left Eye" } else { "OD - Right Eye" };
                if imgs.len() >= 2 {
                    ui.columns(2, |cols| {
                        for (i, col) in cols.iter_mut().enumerate() {
                            let eye = eye_of(i, c.first_is_os);
                            if let Some(Some(t)) = c.thumbs.get(i) {
                                col.add(egui::Image::new((t.id(), t.size_vec2())).max_width(col.available_width()).corner_radius(8.0));
                            }
                            col.add_space(4.0);
                            col.label(egui::RichText::new(eye_name(eye)).font(fonts::semibold(15.0)).color(pal.accent_primary));
                            col.label(egui::RichText::new(imgs[i].as_str()).font(fonts::medium(12.0)).color(pal.text_secondary));
                            col.add_space(6.0);
                            if widgets::button(col, &pal, "Analyse this image only", ButtonKind::Secondary).clicked() {
                                pick = Pick::One(i, eye);
                            }
                        }
                    });
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        if widgets::button(ui, &pal, "Both eyes (Bilateral)", ButtonKind::Primary).clicked() {
                            pick = Pick::Both;
                        }
                        if widgets::button(ui, &pal, "Swap Eyes", ButtonKind::Secondary).clicked() {
                            c.first_is_os = !c.first_is_os;
                        }
                        if widgets::button(ui, &pal, "Cancel", ButtonKind::Ghost).clicked() {
                            pick = Pick::Cancel;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        if let Some(Some(t)) = c.thumbs.first() {
                            ui.add(egui::Image::new((t.id(), t.size_vec2())).max_width(120.0).corner_radius(8.0));
                        }
                        ui.label(egui::RichText::new(imgs.first().map(String::as_str).unwrap_or("")).font(fonts::medium(13.0)).color(pal.text_secondary));
                    });
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        if widgets::button(ui, &pal, "OS - Left Eye", ButtonKind::Primary).clicked() {
                            pick = Pick::One(0, EyeLaterality::OS);
                        }
                        if widgets::button(ui, &pal, "OD - Right Eye", ButtonKind::Primary).clicked() {
                            pick = Pick::One(0, EyeLaterality::OD);
                        }
                        if widgets::button(ui, &pal, "Cancel", ButtonKind::Ghost).clicked() {
                            pick = Pick::Cancel;
                        }
                    });
                }
            });
        if modal.should_close() && matches!(pick, Pick::Wait) {
            pick = Pick::Cancel;
        }
        if matches!(pick, Pick::Wait) {
            return;
        }
        let Some(c) = self.eye_choice.take() else { return };
        let EyeChoice { prompt, mut files, first_is_os, .. } = c;
        match pick {
            Pick::Wait => {}
            Pick::Cancel => {
                self.composer.prompt = prompt;
                self.composer.attachments = files;
            }
            Pick::One(index, eye) => {
                let mut n = 0;
                files.retain(|a| {
                    if a.kind != AttachmentKind::FundusImage {
                        return true;
                    }
                    n += 1;
                    n - 1 == index
                });
                self.exam_view.laterality = eye;
                self.chat_submit(ctx, prompt, files);
            }
            Pick::Both => {
                let mut imgs: Vec<_> = files.iter().filter(|a| a.kind == AttachmentKind::FundusImage).take(2).collect();
                if !first_is_os {
                    imgs.reverse();
                }
                let s = &mut self.bilateral.session;
                s.notice = s.set_mode(Mode::Dual).err();
                for (eye, att) in [Eye::Os, Eye::Od].into_iter().zip(imgs) {
                    let loaded = match validate_attachment_path(&att.source_path) {
                        Ok(valid) => intake::intake(&valid),
                        Err(_) => Err(intake::IntakeError::Blocked),
                    };
                    if let Err(why) = self.bilateral.session.set_source(eye, loaded) {
                        self.bilateral.session.notice = Some(why);
                    }
                }
                self.active_page = AppPage::Bilateral;
            }
        }
    }

    /// Opens the examination that holds this eye's saved result, on the Report page.
    fn bilateral_view_eye(&mut self, ctx: &egui::Context, eye: crate::ui2::bilateral_assessment::state::Eye) {
        let Some(exam) = self.bilateral.session.slot(eye).result.as_ref().map(|r| r.outcome.examination_id).filter(|id| *id > 0) else {
            return;
        };
        self.active_examination_id = Some(exam);
        self.restore_saved_analysis(ctx, exam);
        self.active_page = AppPage::Report;
    }

    fn bilateral_request(&mut self, eyes: &[crate::ui2::bilateral_assessment::state::Eye]) {
        let s = &mut self.bilateral.session;
        s.notice = None;
        if let Err(why) = s.request_run(eyes) {
            s.notice = why.into_iter().next();
        }
    }

    /// Per frame: keep the session bound to the active patient, notice a run that was interrupted
    /// elsewhere, and hand the next queued eye to the cascade when it is idle. Never starts anything
    /// the user did not queue.
    fn bilateral_pump(&mut self, ctx: &egui::Context) {
        let current = self.active_patient_id;
        if self.bilateral.session.patient != current {
            match (self.bilateral.session.patient, current) {
                (None, Some(id)) if self.draft_owner == Some(id) => {
                    self.bilateral.session.adopt_draft_patient(id);
                }
                _ => {
                    if self.bilateral.session.bind_patient(current) {
                        self.reset_analysis_context();
                    }
                }
            }
        }
        let session = &mut self.bilateral.session;
        if let Some(active) = &session.active {
            let live = self.cascade_ctl.active().map(|j| j.request_id);
            if active.request_id.is_some() && live != active.request_id {
                session.reject_result(
                    "The run was interrupted: the image or patient changed in the single-eye workspace.",
                );
            }
        }
        if self.bilateral.session.active.is_some()
            || self.cascade_ctl.is_busy()
            || self.cascade_ctl.finishing_previous()
        {
            return;
        }
        let Some((eye, path)) = self.bilateral.session.next_job() else {
            return;
        };
        use crate::ui2::bilateral_assessment::state::Eye;
        self.exam_view.laterality = match eye {
            Eye::Os => domain::EyeLaterality::OS,
            Eye::Od => domain::EyeLaterality::OD,
        };
        self.exam_view.cascade.error = None;
        self.exam_view.image_path = None;
        self.load_fundus_from_path(ctx, &path);
        if self.exam_view.image_path.is_some() {
            self.start_analysis();
        }
        let expected = self.bilateral.session.active.as_ref().map(|a| a.image_sha256.clone());
        match (self.cascade_ctl.active(), expected) {
            (Some(job), Some(sha)) if job.image_sha256 == sha => {
                let id = job.request_id;
                self.bilateral.session.set_request_id(id);
            }
            (Some(_), _) => {
                self.reset_analysis_context();
                self.bilateral
                    .session
                    .start_failed("The image changed between upload and analysis.".into());
            }
            (None, _) => {
                let why = self
                    .exam_view
                    .cascade
                    .error
                    .clone()
                    .unwrap_or_else(|| "The analysis could not be started.".into());
                self.bilateral.session.start_failed(why);
            }
        }
        ctx.request_repaint();
    }

    /// A cascade run finished. Filed under an eye only when it is the bilateral run in flight and its
    /// request, patient, image hash and eye all match; otherwise the eye is marked failed, never filled.
    fn bilateral_on_finished(&mut self, job: &crate::engine::cascade::JobIdentity) {
        use crate::ui2::bilateral_assessment::state::EyeResult;
        let ours = self
            .bilateral
            .session
            .active
            .as_ref()
            .is_some_and(|a| a.request_id == Some(job.request_id));
        if !ours {
            return;
        }
        let Some(outcome) = self.exam_view.cascade.outcome.clone() else {
            self.bilateral.session.reject_result("The run produced no outcome.");
            return;
        };
        let vessel = self.exam_view.vessel_artifact.texture.as_ref().map(|m| m.texture.clone());
        let l = &self.exam_view.lesion_artifact.textures;
        let lesions: Vec<(&'static str, egui::TextureHandle)> = [
            ("MA", &l.ma),
            ("HE", &l.he),
            ("EX", &l.ex),
            ("CWS", &l.cws),
        ]
        .into_iter()
        .filter_map(|(n, t)| t.as_ref().map(|m| (n, m.texture.clone())))
        .collect();
        let save_note = self.exam_view.cascade.save_note.clone();
        let request_id = job.request_id;
        let verdict = self.bilateral.session.accept_result(
            request_id,
            job.patient_id,
            outcome,
            |outcome| EyeResult {
                outcome,
                request_id,
                vessel,
                lesions,
                save_note,
            },
        );
        if let Err(why) = verdict {
            self.bilateral.session.reject_result(&why);
        }
    }
}

/// Closing the window while an analysis runs asks the backend to stop at its next stage boundary;
/// otherwise dropping the model service would wait for the whole cascade to finish.
impl RetinaCareApp {
    /// Right-hand panel: a source's excerpt, or an attached image / PDF text.
    fn side_panel_ui(&mut self, ui: &mut egui::Ui, rect: Rect, body: Rect) {
        let dark = ui.visuals().dark_mode;
        let (fill, line, text, muted) = if dark {
            (
                Color32::from_rgb(24, 26, 28),
                Color32::from_rgb(52, 55, 60),
                Color32::from_rgb(236, 238, 240),
                Color32::from_rgb(150, 154, 160),
            )
        } else {
            (
                Color32::from_rgb(250, 250, 250),
                Color32::from_rgb(215, 217, 220),
                Color32::from_rgb(40, 42, 46),
                Color32::from_rgb(110, 114, 120),
            )
        };
        ui.painter().rect_filled(rect, 12.0, fill);
        ui.painter()
            .rect_stroke(rect, 12.0, Stroke::new(1.0, line), egui::StrokeKind::Inside);
        let handle = Rect::from_min_max(
            Pos2::new(rect.min.x - 6.0, rect.min.y),
            Pos2::new(rect.min.x + 4.0, rect.max.y),
        );
        let drag = ui
            .interact(handle, Id::new("side_handle"), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if let (true, Some(p)) = (drag.dragged(), drag.interact_pointer_pos()) {
            self.side_frac = ((body.max.x - 8.0 - p.x) / body.width()).clamp(0.25, 0.75);
        }
        let ctx = ui.ctx().clone();
        let Some(side) = self.side.clone() else {
            return;
        };
        let mut close = false;
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(
                rect.shrink2(Vec2::new(14.0, 14.0))
                    .with_max_x(rect.max.x - 24.0),
            ),
        );
        let ui = &mut child;
        let title = match &side {
            SidePanel::Source(s) => s.site.clone(),
            SidePanel::File(a) => a.display_name.clone(),
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(ui::explain::ellipsize(&title, 40))
                    .size(13.0)
                    .strong()
                    .color(text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        Button::new(RichText::new("\u{2715}").size(13.0).color(muted)).frame(false),
                    )
                    .on_hover_text("Close")
                    .clicked()
                {
                    close = true;
                }
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt("side_scroll").auto_shrink([false, false]).show(ui, |ui| match &side {
            SidePanel::Source(site) => {
                ui.label(RichText::new(&site.title).size(17.0).strong().color(text));
                ui.add_space(4.0);
                ui.hyperlink_to(RichText::new(&site.url).size(11.5), &site.url);
                ui.add_space(10.0);
                ui.label(RichText::new("What the assistant read from this page").size(11.5).color(muted));
                ui.add_space(2.0);
                ui.label(RichText::new(&site.snippet).size(14.0).color(text));
                ui.add_space(12.0);
                if ui.button("Open in browser").clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(&site.url));
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Only the excerpt is shown here. A full in-app browser is not available on this system.")
                        .size(11.0)
                        .color(muted),
                );
            }
            SidePanel::File(att) => match att.kind {
                AttachmentKind::FundusImage => {
                    if self.side_tex.as_ref().is_none_or(|(p, _)| p != &att.source_path) {
                        self.side_tex = image::open(&att.source_path).ok().map(|img| {
                            let img = img.thumbnail(1600, 1600).to_rgba8();
                            let size = [img.width() as usize, img.height() as usize];
                            let tex = ctx.load_texture("side_image", egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), Default::default());
                            (att.source_path.clone(), tex)
                        });
                    }
                    match &self.side_tex {
                        Some((_, tex)) => {
                            let w = ui.available_width();
                            let s = tex.size_vec2();
                            ui.image((tex.id(), Vec2::new(w, w * s.y / s.x.max(1.0))));
                        }
                        None => {
                            ui.label(RichText::new("This image can no longer be opened.").color(muted));
                        }
                    }
                }
                _ => {
                    if self.side_text.as_ref().is_none_or(|(p, _)| p != &att.source_path) {
                        let body = lopdf::Document::load(&att.source_path)
                            .ok()
                            .map(|d| {
                                let pages: Vec<u32> = d.get_pages().keys().copied().take(4).collect();
                                d.extract_text(&pages).unwrap_or_default().chars().take(8000).collect::<String>()
                            })
                            .filter(|t| !t.trim().is_empty())
                            .unwrap_or_else(|| "No text could be read from this PDF.".into());
                        self.side_text = Some((att.source_path.clone(), body));
                    }
                    if let Some((_, body)) = &self.side_text {
                        ui.label(RichText::new("Text of the first pages").size(11.5).color(muted));
                        ui.add_space(4.0);
                        ui.label(RichText::new(body).size(13.0).color(text));
                    }
                }
            },
        });
        if close {
            self.side = None;
        }
    }

    /// Asks once where a source should open; "remember" turns the answer into the default.
    fn open_choice_ui(&mut self, ctx: &egui::Context) {
        let Some(site) = self.pending_site.clone() else {
            return;
        };
        let remember_id = Id::new("remember_open_choice");
        let mut remember = ctx
            .data(|d| d.get_temp::<bool>(remember_id))
            .unwrap_or(false);
        let mut open = true;
        let mut choice = None;
        egui::Window::new("Open source")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new(&site.title).strong());
                ui.label(
                    RichText::new(&site.site)
                        .size(11.5)
                        .color(theme::TEXT_MUTED()),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Open in browser").clicked() {
                        choice = Some(false);
                    }
                    if ui.button("Open in app").clicked() {
                        choice = Some(true);
                    }
                });
                ui.add_space(6.0);
                ui.checkbox(&mut remember, "Remember my choice");
            });
        ctx.data_mut(|d| d.insert_temp(remember_id, remember));
        if let Some(in_app) = choice {
            if remember {
                self.open_pref = Some(in_app);
            }
            if in_app {
                self.side = Some(SidePanel::Source(site));
            } else {
                ctx.open_url(egui::OpenUrl::new_tab(&site.url));
            }
            self.pending_site = None;
        } else if !open {
            self.pending_site = None;
        }
    }
}

impl Drop for RetinaCareApp {
    fn drop(&mut self) {
        if let Some(id) = self.cascade_ctl.abandon() {
            self.backend_service.cancel_cascade(id);
        }
    }
}

impl eframe::App for RetinaCareApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.window_icon_sent {
            // Set the icon once the window exists; some Linux window managers ignore the creation-time icon.
            self.window_icon_sent = true;
            if let Some(icon) = app_window_icon() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(icon))));
            }
        }

        // Low-frequency heartbeat (500ms): Wakes the event loop periodically even during prolonged
        // idle periods. Prevents Wayland compositor surface frame throttling and keeps input event
        // queues pumped so that clicks after long idle periods register immediately without unresponsiveness.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        self.poll_explain(&ctx);
        self.save_dialog_ui(&ctx);
        self.chat_sync(&ctx);
        self.cloud_poll(&ctx);
        if std::mem::take(&mut self.exam_view.explain_link_clicked) {
            self.active_page = AppPage::Explainability;
        }
        if std::mem::take(&mut self.exam_view.lesion_link_clicked) {
            self.active_page = AppPage::LesionAnalysis;
        }
        if std::mem::take(&mut self.exam_view.vessel_link_clicked) {
            self.active_page = AppPage::VesselAnalysis;
        }

        // 0. Drain nonblocking background chat events
        let pending_chat_events: Vec<ChatEvent> = self
            .chat_event_rx
            .as_ref()
            .map(|rx| {
                std::iter::from_fn(|| rx.try_recv().ok())
                    .take(256)
                    .collect()
            })
            .unwrap_or_default();
        {
            for event in pending_chat_events {
                match event {
                    ChatEvent::Started { request_id, .. } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.status = ChatMessageStatus::Streaming;
                        }
                    }
                    ChatEvent::Sources {
                        request_id,
                        sources,
                        note,
                    } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            if !sources.is_empty() {
                                msg.sources = sources;
                            }
                            msg.source_note = note;
                        }
                    }
                    ChatEvent::Web {
                        request_id,
                        searching,
                        query,
                        sites,
                    } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            let started = msg
                                .web
                                .as_ref()
                                .map_or_else(std::time::Instant::now, |w| w.started);
                            msg.web = Some(domain::WebLookup {
                                searching,
                                query,
                                sites,
                                started,
                                took_secs: started.elapsed().as_secs(),
                            });
                        }
                        ctx.request_repaint();
                    }
                    ChatEvent::Delta { request_id, text } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.append_delta(&text);
                        }
                        ctx.request_repaint();
                    }
                    ChatEvent::Completed { request_id } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.complete();
                            if let Some(w) = msg.web.as_mut() {
                                w.took_secs = w.started.elapsed().as_secs();
                            }
                        }
                        self.persist_final(&request_id);
                        if self.active_request_id.as_deref() == Some(&request_id) {
                            self.active_request_id = None;
                            self.composer.is_busy = false;
                        }
                        ctx.request_repaint();
                    }
                    ChatEvent::Failed { request_id, error } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.fail(error);
                        }
                        self.persist_final(&request_id);
                        if self.active_request_id.as_deref() == Some(&request_id) {
                            self.active_request_id = None;
                            self.composer.is_busy = false;
                        }
                        ctx.request_repaint();
                    }
                    ChatEvent::Cancelled { request_id } => {
                        if let Some(msg) = self
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.cancel();
                        }
                        self.persist_final(&request_id);
                        if self.active_request_id.as_deref() == Some(&request_id) {
                            self.active_request_id = None;
                            self.composer.is_busy = false;
                        }
                        ctx.request_repaint();
                    }
                }
            }
        }

        // 0a. Drain nonblocking Stage I -> II -> III cascade events (separate channel).
        self.refresh_cascade_availability();
        self.drain_cascade_events(&ctx);
        self.bilateral_pump(&ctx);
        self.eye_choice_window(&ctx);

        // 0b. Drain nonblocking medical inference backend events
        while let Some(event) = self.backend_service.try_recv() {
            match event {
                crate::engine::InferenceEvent::ReadinessChanged(readiness) => {
                    self.backend_readiness = readiness;
                    ctx.request_repaint();
                }
                crate::engine::InferenceEvent::Progress { .. } => {
                    ctx.request_repaint();
                }
                crate::engine::InferenceEvent::Stage1Completed(result) => {
                    if result.examination_id == self.exam_view.exam_id {
                        self.stage1_result = Some(result);
                        self.stage1_error = None;
                    } else {
                        eprintln!("Stale inference event rejected: examination mismatch");
                    }
                    self.backend_readiness = self.backend_service.readiness();
                    self.active_screening_request = None;
                    ctx.request_repaint();
                }
                crate::engine::InferenceEvent::InferenceFailed {
                    request_id: _,
                    error,
                } => {
                    self.stage1_error = Some(error);
                    self.backend_readiness = self.backend_service.readiness();
                    self.active_screening_request = None;
                    ctx.request_repaint();
                }
                crate::engine::InferenceEvent::Cancelled { .. } => {
                    self.stage1_error = Some("Inference screening cancelled by user.".to_string());
                    self.backend_readiness = self.backend_service.readiness();
                    self.active_screening_request = None;
                    ctx.request_repaint();
                }
            }
        }

        // 0c. Drain nonblocking PDF and report export background events
        if let Some(rx) = &self.export_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::engine::ExportEvent::Started { request_id } => {
                        self.export_state = crate::engine::ExportState::Generating { request_id };
                        ctx.request_repaint();
                    }
                    crate::engine::ExportEvent::Progress { .. } => {
                        ctx.request_repaint();
                    }
                    crate::engine::ExportEvent::Completed {
                        request_id,
                        path,
                        bytes,
                    } => {
                        self.export_state = crate::engine::ExportState::Saved {
                            request_id,
                            path: path.clone(),
                            bytes,
                        };
                        let _ = self
                            .audit_ledger_service
                            .record_event(crate::engine::AuditEvent {
                                event_id: format!(
                                    "evt_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0)
                                ),
                                timestamp: crate::engine::report::utc_now(),
                                event_type: crate::engine::AuditEventType::PdfExportCompleted,
                                examination_id: self.exam_view.exam_id.clone(),
                                request_id: "req_pdf_export".to_string(),
                                operator: current_operator(),
                                status: crate::engine::AuditEventStatus::Success,
                                non_sensitive_summary: format!(
                                "Clinical report PDF export successfully written ({bytes} bytes)"
                            ),
                            });
                        self.cached_audit_events = self
                            .audit_ledger_service
                            .query_events(Some(&self.exam_view.exam_id))
                            .unwrap_or_default();
                        ctx.request_repaint();
                    }
                    crate::engine::ExportEvent::Failed { request_id, error } => {
                        self.export_state =
                            crate::engine::ExportState::Failed { request_id, error };
                        let _ = self
                            .audit_ledger_service
                            .record_event(crate::engine::AuditEvent {
                                event_id: format!(
                                    "evt_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0)
                                ),
                                timestamp: crate::engine::report::utc_now(),
                                event_type: crate::engine::AuditEventType::ExportFailed,
                                examination_id: self.exam_view.exam_id.clone(),
                                request_id: "req_pdf_export".to_string(),
                                operator: current_operator(),
                                status: crate::engine::AuditEventStatus::Failed,
                                non_sensitive_summary: "Clinical report PDF export failed"
                                    .to_string(),
                            });
                        self.cached_audit_events = self
                            .audit_ledger_service
                            .query_events(Some(&self.exam_view.exam_id))
                            .unwrap_or_default();
                        ctx.request_repaint();
                    }
                    crate::engine::ExportEvent::Cancelled { request_id } => {
                        self.export_state = crate::engine::ExportState::Cancelled { request_id };
                        ctx.request_repaint();
                    }
                }
            }
        }

        // New design (RETINACARE_UI=v2) draws its own chrome; the old chrome is kept until approval.
        let (body_rect, sidebar_width) = if crate::ui2::theme::enabled() {
            self.chrome_v2(ui, &ctx)
        } else {
            let screen_rect = ui.max_rect();

            // 1. Root Dark Outer Shell (#101113)
            ui.painter().rect(
                screen_rect,
                CornerRadius::same(12),
                theme::OUTER_FRAME(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            let inner_rect = screen_rect.shrink(8.0);

            // 2. Custom Title Bar (#292B2A - Height: 38px, native window drag & controls per New Image 1 & 2)
            let title_bar_rect =
                Rect::from_min_size(inner_rect.min, Vec2::new(inner_rect.width(), 38.0));
            ui.painter().rect(
                title_bar_rect,
                CornerRadius {
                    nw: 10,
                    ne: 10,
                    sw: 0,
                    se: 0,
                },
                theme::TOP_BAR_BG(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            // Sidebar Toggle Button on top-left of Title Bar (exact placement per Image 1 & Image 2)
            let toggle_rect = Rect::from_min_size(
                Pos2::new(title_bar_rect.min.x + 8.0, title_bar_rect.min.y + 5.0),
                Vec2::new(28.0, 28.0),
            );
            let toggle_response = ui.interact(
                toggle_rect,
                Id::new("sidebar_toggle_button"),
                egui::Sense::click(),
            );
            let toggle_hovered = toggle_response.hovered();
            let toggle_bg = if toggle_hovered {
                Color32::from_rgba_unmultiplied(255, 255, 255, 24)
            } else {
                theme::CARD_BG()
            };
            ui.painter().rect(
                toggle_rect,
                CornerRadius::same(6),
                toggle_bg,
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            let toggle_center = toggle_rect.center();
            let toggle_icon_stroke = Stroke::new(
                1.4,
                if toggle_hovered {
                    Color32::WHITE
                } else {
                    theme::TEXT_SECONDARY()
                },
            );
            ui.painter().rect_stroke(
                Rect::from_center_size(toggle_center, Vec2::new(14.0, 12.0)),
                CornerRadius::same(2),
                toggle_icon_stroke,
                StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(toggle_center.x - 2.0, toggle_center.y - 6.0),
                    Pos2::new(toggle_center.x - 2.0, toggle_center.y + 6.0),
                ],
                toggle_icon_stroke,
            );
            if toggle_response.clicked() {
                self.sidebar_open = !self.sidebar_open;
                ctx.request_repaint();
            }

            // Title bar drag area across the bar (between toggle button and window controls)
            let title_drag_rect = Rect::from_min_max(
                Pos2::new(toggle_rect.max.x + 8.0, title_bar_rect.min.y),
                Pos2::new(title_bar_rect.max.x - 130.0, title_bar_rect.max.y),
            );
            let drag_resp = ui.interact(
                title_drag_rect,
                Id::new("win_drag_bar"),
                egui::Sense::drag(),
            );
            if drag_resp.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            // Window Controls (Simple glyphs, not inside a box when idle - per Image 3)
            let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            let btn_width = 34.0;
            let btn_height = 28.0;
            let btns_top = title_bar_rect.min.y + 5.0;

            // Close button (X)
            let close_rect = Rect::from_min_size(
                Pos2::new(title_bar_rect.max.x - btn_width - 6.0, btns_top),
                Vec2::new(btn_width, btn_height),
            );
            let close_resp =
                ui.interact(close_rect, Id::new("win_close_btn"), egui::Sense::click());
            if close_resp.hovered() {
                ui.painter().rect_filled(
                    close_rect,
                    CornerRadius::same(6),
                    Color32::from_rgb(220, 38, 38),
                );
            }
            let close_center = close_rect.center();
            let close_stroke = Stroke::new(
                1.6,
                if close_resp.hovered() {
                    Color32::WHITE
                } else {
                    theme::TEXT_SECONDARY()
                },
            );
            ui.painter().line_segment(
                [
                    Pos2::new(close_center.x - 4.5, close_center.y - 4.5),
                    Pos2::new(close_center.x + 4.5, close_center.y + 4.5),
                ],
                close_stroke,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(close_center.x + 4.5, close_center.y - 4.5),
                    Pos2::new(close_center.x - 4.5, close_center.y + 4.5),
                ],
                close_stroke,
            );
            if close_resp.clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }

            // Maximize button ([])
            let max_rect = Rect::from_min_size(
                Pos2::new(close_rect.min.x - btn_width - 4.0, btns_top),
                Vec2::new(btn_width, btn_height),
            );
            let max_resp = ui.interact(max_rect, Id::new("win_max_btn"), egui::Sense::click());
            if max_resp.hovered() {
                ui.painter().rect_filled(
                    max_rect,
                    CornerRadius::same(6),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                );
            }
            let max_center = max_rect.center();
            let max_stroke = Stroke::new(1.4, theme::TEXT_SECONDARY());
            ui.painter().rect_stroke(
                Rect::from_center_size(max_center, Vec2::new(10.0, 10.0)),
                CornerRadius::same(2),
                max_stroke,
                StrokeKind::Inside,
            );
            if max_resp.clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
            }

            // Minimize button (_)
            let min_rect = Rect::from_min_size(
                Pos2::new(max_rect.min.x - btn_width - 4.0, btns_top),
                Vec2::new(btn_width, btn_height),
            );
            let min_resp = ui.interact(min_rect, Id::new("win_min_btn"), egui::Sense::click());
            if min_resp.hovered() {
                ui.painter().rect_filled(
                    min_rect,
                    CornerRadius::same(6),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                );
            }
            let min_center = min_rect.center();
            ui.painter().line_segment(
                [
                    Pos2::new(min_center.x - 5.0, min_center.y + 4.0),
                    Pos2::new(min_center.x + 5.0, min_center.y + 4.0),
                ],
                Stroke::new(1.6, theme::TEXT_SECONDARY()),
            );
            if min_resp.clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }

            // 3. Main Body Canvas: #191919 (Exact Image 1 & 2 sampled color)
            let body_rect = Rect::from_min_max(
                Pos2::new(inner_rect.min.x, inner_rect.min.y + 38.0),
                inner_rect.max,
            );
            ui.painter().rect(
                body_rect,
                CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 10,
                    se: 10,
                },
                theme::WORKSPACE(),
                Stroke::NONE,
                StrokeKind::Inside,
            );

            // 4. Smooth Animated Sidebar (#1F1F1F - Exact Image 3 sampled color)
            let sidebar_t = ui.ctx().animate_bool_with_time_and_easing(
                Id::new("sidebar_smooth_anim"),
                self.sidebar_open,
                0.28,
                egui::emath::easing::cubic_out,
            );
            if sidebar_t > 0.001 && sidebar_t < 0.999 {
                ui.ctx().request_repaint();
            }

            let max_sidebar_width = 240.0; // Slim & balanced width per user preference
            let sidebar_width = sidebar_t * max_sidebar_width;

            if sidebar_width > 0.5 {
                let sidebar_rect = Rect::from_min_size(
                    body_rect.min,
                    Vec2::new(sidebar_width, body_rect.height()),
                );
                ui.painter().rect(
                    sidebar_rect,
                    CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 10,
                        se: 0,
                    },
                    theme::SIDEBAR(),
                    Stroke::new(1.0, theme::BORDER()),
                    StrokeKind::Inside,
                );

                let sidebar_painter = ui.painter().with_clip_rect(sidebar_rect);

                // Left Sidebar Page Navigation: Home, Report, Patients, Previous Chats, Timeline & Plugins
                let nav_pages = [
                    (AppPage::Home, "Home"),
                    (AppPage::Report, "Report"),
                    (AppPage::Patients, "Patients"),
                    (AppPage::PreviousChats, "Previous Chats"),
                    (AppPage::Timeline, "Timeline"),
                    (AppPage::Explainability, "Explainability"),
                    (AppPage::LesionAnalysis, "Lesion Analysis"),
                    (AppPage::VesselAnalysis, "Vessel Analysis"),
                    (AppPage::ResearchFindings, "Research Findings"),
                ];
                let nav_start_y = body_rect.min.y + 16.0;
                let nav_item_gap = 10.0;
                let nav_item_height = 38.0;

                for (i, (page, label)) in nav_pages.iter().enumerate() {
                    let nav_y = nav_start_y + (i as f32) * (nav_item_height + nav_item_gap);
                    let is_active = self.active_page == *page;
                    let nav_rect = Rect::from_min_size(
                        Pos2::new(body_rect.min.x + 10.0, nav_y),
                        Vec2::new((sidebar_width - 20.0).max(0.0), nav_item_height),
                    );

                    if nav_rect.width() > 20.0 {
                        let nav_resp = ui.interact(
                            nav_rect,
                            Id::new("nav_page_btn").with(i),
                            egui::Sense::click(),
                        );
                        let is_hovered = nav_resp.hovered();

                        if is_active {
                            sidebar_painter.rect_filled(
                                nav_rect,
                                CornerRadius::same(8),
                                Color32::from_rgb(38, 40, 44),
                            );
                            sidebar_painter.rect_stroke(
                                nav_rect,
                                CornerRadius::same(8),
                                Stroke::new(1.0, theme::BORDER_LIGHT()),
                                StrokeKind::Inside,
                            );
                        } else if is_hovered {
                            sidebar_painter.rect_filled(
                                nav_rect,
                                CornerRadius::same(8),
                                theme::CARD_HOVER(),
                            );
                        }

                        // Clean icon
                        let icon_center = Pos2::new(nav_rect.min.x + 22.0, nav_rect.center().y);
                        let icon_color = if is_active {
                            theme::TEXT_PRIMARY()
                        } else {
                            theme::TEXT_SECONDARY()
                        };
                        let stroke = Stroke::new(1.4, icon_color);

                        match page {
                            AppPage::Home => {
                                // Home icon [⌂]
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x - 6.0, icon_center.y - 1.0),
                                        Pos2::new(icon_center.x, icon_center.y - 7.0),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x, icon_center.y - 7.0),
                                        Pos2::new(icon_center.x + 6.0, icon_center.y - 1.0),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.rect_stroke(
                                    Rect::from_center_size(
                                        Pos2::new(icon_center.x, icon_center.y + 3.0),
                                        Vec2::new(9.0, 7.0),
                                    ),
                                    CornerRadius::same(1),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                            }
                            AppPage::Report | AppPage::ReportDoc | AppPage::Assessment => {
                                // Document/Report icon [📄]
                                let doc_rect =
                                    Rect::from_center_size(icon_center, Vec2::new(11.0, 13.0));
                                sidebar_painter.rect_stroke(
                                    doc_rect,
                                    CornerRadius::same(2),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(doc_rect.min.x + 2.5, doc_rect.min.y + 3.5),
                                        Pos2::new(doc_rect.max.x - 2.5, doc_rect.min.y + 3.5),
                                    ],
                                    Stroke::new(1.2, icon_color),
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(doc_rect.min.x + 2.5, doc_rect.min.y + 6.5),
                                        Pos2::new(doc_rect.max.x - 2.5, doc_rect.min.y + 6.5),
                                    ],
                                    Stroke::new(1.2, icon_color),
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(doc_rect.min.x + 2.5, doc_rect.min.y + 9.5),
                                        Pos2::new(doc_rect.max.x - 4.5, doc_rect.min.y + 9.5),
                                    ],
                                    Stroke::new(1.2, icon_color),
                                );
                            }
                            AppPage::Patients => {
                                // Medical patient identity icon: user head circle & shoulder curve
                                let head_center = Pos2::new(icon_center.x, icon_center.y - 3.5);
                                sidebar_painter.circle_stroke(head_center, 3.2, stroke);
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x - 6.0, icon_center.y + 6.0),
                                        Pos2::new(icon_center.x - 3.0, icon_center.y + 2.0),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x - 3.0, icon_center.y + 2.0),
                                        Pos2::new(icon_center.x + 3.0, icon_center.y + 2.0),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x + 3.0, icon_center.y + 2.0),
                                        Pos2::new(icon_center.x + 6.0, icon_center.y + 6.0),
                                    ],
                                    stroke,
                                );
                            }
                            AppPage::PreviousChats => {
                                // Chat speech bubble icon [💬]
                                let bubble_rect = Rect::from_center_size(
                                    Pos2::new(icon_center.x, icon_center.y - 1.0),
                                    Vec2::new(12.0, 10.0),
                                );
                                sidebar_painter.rect_stroke(
                                    bubble_rect,
                                    CornerRadius::same(2),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(bubble_rect.min.x + 2.5, bubble_rect.max.y),
                                        Pos2::new(bubble_rect.min.x + 0.5, bubble_rect.max.y + 3.0),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(bubble_rect.min.x + 0.5, bubble_rect.max.y + 3.0),
                                        Pos2::new(bubble_rect.min.x + 5.0, bubble_rect.max.y),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(bubble_rect.min.x + 2.5, bubble_rect.center().y),
                                        Pos2::new(bubble_rect.max.x - 2.5, bubble_rect.center().y),
                                    ],
                                    Stroke::new(1.0, icon_color),
                                );
                            }
                            AppPage::Timeline => {
                                // Clinical Timeline icon [⏱ / ──●──]: Clock circle + timeline axis
                                let clock_c = Pos2::new(icon_center.x, icon_center.y - 1.5);
                                sidebar_painter.circle_stroke(clock_c, 5.0, stroke);
                                // Clock hands
                                sidebar_painter.line_segment(
                                    [clock_c, Pos2::new(clock_c.x, clock_c.y - 3.0)],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [clock_c, Pos2::new(clock_c.x + 2.5, clock_c.y)],
                                    stroke,
                                );
                                // Lower horizontal timeline track
                                let track_y = icon_center.y + 6.0;
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(icon_center.x - 7.0, track_y),
                                        Pos2::new(icon_center.x + 7.0, track_y),
                                    ],
                                    Stroke::new(1.2, icon_color),
                                );
                                // Milestone node on track
                                sidebar_painter.circle_filled(
                                    Pos2::new(icon_center.x + 2.0, track_y),
                                    2.0,
                                    icon_color,
                                );
                            }
                            AppPage::Explainability => {
                                // Attribution icon: concentric heat rings
                                let c = Pos2::new(icon_center.x, icon_center.y);
                                sidebar_painter.circle_stroke(c, 7.0, stroke);
                                sidebar_painter.circle_stroke(
                                    Pos2::new(c.x + 1.5, c.y - 1.0),
                                    3.8,
                                    stroke,
                                );
                                sidebar_painter.circle_filled(
                                    Pos2::new(c.x + 1.5, c.y - 1.0),
                                    1.4,
                                    icon_color,
                                );
                            }
                            AppPage::LesionAnalysis => {
                                // Lesion channels: four dots in a ring
                                for (dx, dy) in [(-4.0, -4.0), (4.0, -4.0), (-4.0, 4.0), (4.0, 4.0)]
                                {
                                    sidebar_painter.circle_filled(
                                        Pos2::new(icon_center.x + dx, icon_center.y + dy),
                                        2.2,
                                        icon_color,
                                    );
                                }
                                sidebar_painter.circle_stroke(icon_center, 7.5, stroke);
                            }
                            AppPage::VesselAnalysis => {
                                // Branching vessel
                                let c = icon_center;
                                sidebar_painter.line_segment(
                                    [Pos2::new(c.x - 7.0, c.y + 6.0), Pos2::new(c.x, c.y)],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [Pos2::new(c.x, c.y), Pos2::new(c.x + 7.0, c.y - 6.0)],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [Pos2::new(c.x, c.y), Pos2::new(c.x + 7.0, c.y + 5.0)],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(c.x + 3.5, c.y - 3.0),
                                        Pos2::new(c.x - 2.0, c.y - 7.0),
                                    ],
                                    stroke,
                                );
                            }
                            AppPage::ResearchFindings => {
                                // Evidence sheet with a bar chart
                                let r = Rect::from_center_size(icon_center, Vec2::new(13.0, 14.0));
                                sidebar_painter.rect_stroke(
                                    r,
                                    CornerRadius::same(2),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                                for (i, h) in [3.0_f32, 6.0, 4.5].iter().enumerate() {
                                    let x = r.min.x + 3.0 + i as f32 * 3.4;
                                    sidebar_painter.line_segment(
                                        [
                                            Pos2::new(x, r.max.y - 2.5),
                                            Pos2::new(x, r.max.y - 2.5 - h),
                                        ],
                                        Stroke::new(1.6, icon_color),
                                    );
                                }
                            }
                            AppPage::Plugins => {
                                // Modular Plugin icon [🧩]
                                let plug_rect = Rect::from_center_size(
                                    Pos2::new(icon_center.x, icon_center.y + 1.0),
                                    Vec2::new(11.0, 10.0),
                                );
                                sidebar_painter.rect_stroke(
                                    plug_rect,
                                    CornerRadius::same(2),
                                    stroke,
                                    StrokeKind::Inside,
                                );
                                // Top connector pins
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(plug_rect.center().x - 2.5, plug_rect.min.y),
                                        Pos2::new(
                                            plug_rect.center().x - 2.5,
                                            plug_rect.min.y - 3.0,
                                        ),
                                    ],
                                    stroke,
                                );
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(plug_rect.center().x + 2.5, plug_rect.min.y),
                                        Pos2::new(
                                            plug_rect.center().x + 2.5,
                                            plug_rect.min.y - 3.0,
                                        ),
                                    ],
                                    stroke,
                                );
                                // Module detail line
                                sidebar_painter.line_segment(
                                    [
                                        Pos2::new(plug_rect.min.x + 2.5, plug_rect.center().y),
                                        Pos2::new(plug_rect.max.x - 2.5, plug_rect.center().y),
                                    ],
                                    Stroke::new(1.0, icon_color),
                                );
                            }
                            // Reachable only from the v2 sidebar; the legacy list above never offers it.
                            AppPage::RetinalImaging | AppPage::Bilateral => {}
                        }

                        // Label text
                        sidebar_painter.text(
                            Pos2::new(nav_rect.min.x + 40.0, nav_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            *label,
                            egui::FontId::proportional(13.0),
                            if is_active {
                                theme::TEXT_PRIMARY()
                            } else {
                                theme::TEXT_SECONDARY()
                            },
                        );

                        if nav_resp.clicked() {
                            if *page == AppPage::Home {
                                self.chat_messages.clear();
                                self.active_session_id = None;
                                self.is_chat_docked = false;
                            }
                            self.active_page = *page;
                            ctx.request_repaint();
                        }
                    }
                }
            }

            (body_rect, sidebar_width)
        };

        // 5. Main Workspace Area
        let main_min_x = body_rect.min.x + sidebar_width + 16.0;
        let side_w = if self.side.is_some() && self.active_page == AppPage::Home {
            (body_rect.width() * self.side_frac)
                .clamp(260.0, body_rect.width() - sidebar_width - 320.0)
        } else {
            0.0
        };
        let main_max_x = body_rect.max.x - 16.0 - if side_w > 0.0 { side_w + 8.0 } else { 0.0 };
        if side_w > 0.0 {
            let side_rect = Rect::from_min_max(
                Pos2::new(body_rect.max.x - side_w - 8.0, body_rect.min.y + 12.0),
                Pos2::new(body_rect.max.x - 8.0, body_rect.max.y - 12.0),
            );
            self.side_panel_ui(ui, side_rect, body_rect);
        }
        let main_min_y = body_rect.min.y + 12.0;
        let main_max_y = body_rect.max.y - 12.0;

        let main_rect = Rect::from_min_max(
            Pos2::new(main_min_x, main_min_y),
            Pos2::new(main_max_x, main_max_y),
        );

        // The paper view is rebuilt from current results each time it is opened.
        if self.active_page != AppPage::ReportDoc {
            self.report_doc = None;
        }
        // Route between Home page and Report page
        match self.active_page {
            AppPage::Home => {
                // Smooth Docking Animation: 0.0 = Centered Initial State, 1.0 = Bottom Docked State
                let dock_t = ui.ctx().animate_bool_with_time_and_easing(
                    Id::new("composer_dock_anim"),
                    self.is_chat_docked,
                    0.35,
                    egui::emath::easing::cubic_out,
                );
                if dock_t > 0.001 && dock_t < 0.999 {
                    ui.ctx().request_repaint();
                }

                let v2 = crate::ui2::theme::enabled();
                let composer_h = if v2 {
                    crate::ui2::composer::block_height(
                        !self.composer.attachments.is_empty(),
                        dock_t > 0.5,
                    )
                } else if self.composer.attachments.is_empty() {
                    166.0
                } else {
                    212.0
                };
                let center_top = ((main_rect.height() - composer_h) / 2.0).max(30.0);
                let docked_top = (main_rect.height() - composer_h - 14.0).max(40.0);
                let comp_top = center_top * (1.0 - dock_t) + docked_top * dock_t;

                // Render Chat Message Feed above composer when docked or docking
                if dock_t > 0.01 {
                    let feed_rect = Rect::from_min_max(
                        Pos2::new(main_rect.min.x, main_rect.min.y),
                        Pos2::new(main_rect.max.x, main_rect.min.y + comp_top - 12.0),
                    );

                    if feed_rect.height() > 40.0 {
                        let mut feed_ui = ui.new_child(egui::UiBuilder::new().max_rect(feed_rect));
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .auto_shrink([false, false])
                            .show(&mut feed_ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    let feed_width = 880.0_f32.min(ui.available_width() - 48.0);
                                    ui.set_width(feed_width);

                                    // Top breathing room padding
                                    ui.add_space(16.0);

                                    let mut toggle_expand_idx = None;
                                    let mut retry_trigger_idx = None;
                                    let mut open_source: Option<crate::domain::SourceRef> = None;
                                    let mut pick_site: Option<domain::WebSite> = None;
                                    let mut pick_file: Option<domain::LocalAttachment> = None;
                                    let chat_style = self.chat_style;
                                    let mut cancel_trigger_req = None;
                                    let mut card_link: Option<ui::chat_cards::CardLink> = None;
                                    if let Some(note) = &self.chat_notice {
                                        ui.label(
                                            RichText::new(note)
                                                .size(11.5)
                                                .color(theme::DIR_AMBER()),
                                        );
                                        ui.add_space(8.0);
                                    }
                                    let run_on_screen = self
                                        .exam_view
                                        .cascade
                                        .outcome
                                        .as_ref()
                                        .filter(|o| o.identity_verified)
                                        .map(|o| o.request_id);
                                    let card_textures = ui::chat_cards::CardTextures {
                                        run_id: run_on_screen,
                                        exam: self.active_examination_id,
                                        draft_owner: self.draft_owner,
                                        patient: self
                                            .active_session_id
                                            .as_deref()
                                            .and_then(|id| self.previous_sessions.iter().find(|s| s.id == id))
                                            .and_then(|s| s.patient_id),
                                        original: self.mode_textures[0].as_ref(),
                                        masks: [
                                            self.exam_view.lesion_artifact.textures.ma.as_ref().map(|m| &m.texture),
                                            self.exam_view.lesion_artifact.textures.he.as_ref().map(|m| &m.texture),
                                            self.exam_view.lesion_artifact.textures.ex.as_ref().map(|m| &m.texture),
                                            self.exam_view.lesion_artifact.textures.cws.as_ref().map(|m| &m.texture),
                                            self.exam_view.vessel_artifact.texture.as_ref().map(|m| &m.texture),
                                        ],
                                    };

                                    for (idx, msg) in self.chat_messages.iter().enumerate() {
                                        if let Some(card) = &msg.card {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    if let Some(l) = ui::chat_cards::render_analysis_card(
                                                        ui,
                                                        msg,
                                                        card,
                                                        &self.exam_view.cascade,
                                                        &card_textures,
                                                        feed_width,
                                                    ) {
                                                        card_link = Some(l);
                                                    }
                                                });
                                            });
                                            ui.add_space(22.0);
                                            continue;
                                        }
                                        match msg.sender {
                                            MessageSender::User => {
                                                // Dynamic Content-Adaptive Width (Fixes stretched huge box for short text like "hi")
                                                let longest_line_len = msg
                                                    .text
                                                    .lines()
                                                    .map(|l| l.chars().count())
                                                    .max()
                                                    .unwrap_or(0);
                                                let char_px = 8.5;
                                                let text_content_w = (longest_line_len as f32 * char_px)
                                                    .clamp(24.0, feed_width * 0.72);
                                                let min_bubble_w = 160.0_f32;
                                                let att_w = if msg.attachments.is_empty() {
                                                    0.0
                                                } else {
                                                    220.0
                                                };
                                                let bubble_width = (text_content_w + 48.0)
                                                    .max(min_bubble_w)
                                                    .max(att_w)
                                                    .min(feed_width * 0.78);

                                                ui.horizontal(|ui| {
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Min),
                                                        |ui| {
                                                            let card_frame = Frame::new()
                                                                .fill(theme::CONTROL_RAISED())
                                                                .stroke(Stroke::new(
                                                                    1.0,
                                                                    theme::BORDER(),
                                                                ))
                                                                .corner_radius(CornerRadius::same(12))
                                                                .inner_margin(Margin::symmetric(16, 12));

                                                            card_frame.show(ui, |ui| {
                                                                ui.set_width(bubble_width);
                                                                ui.vertical(|ui| {
                                                                    // Top row: You label, timestamp, copy button
                                                                    ui.horizontal(|ui| {
                                                                        ui.label(
                                                                            RichText::new("You")
                                                                                .size(12.5)
                                                                                .strong()
                                                                                .color(theme::TEXT_PRIMARY()),
                                                                        );
                                                                        ui.add_space(8.0);
                                                                        ui.label(
                                                                            RichText::new(&msg.timestamp)
                                                                                .size(11.0)
                                                                                .color(theme::TEXT_MUTED()),
                                                                        );

                                                                        ui.with_layout(
                                                                            Layout::right_to_left(Align::Center),
                                                                            |ui| {
                                                                                let copy_btn = ui.add(
                                                                                    Button::new(
                                                                                        RichText::new("📋")
                                                                                            .size(11.0)
                                                                                            .color(theme::TEXT_SECONDARY()),
                                                                                    )
                                                                                    .frame(false),
                                                                                );
                                                                                if copy_btn.clicked() {
                                                                                    ui.ctx().copy_text(msg.text.clone());
                                                                                }
                                                                            },
                                                                        );
                                                                    });

                                                                    // Attachment cards inside user card
                                                                    if !msg.attachments.is_empty() {
                                                                        ui.add_space(6.0);
                                                                        match msg.attachments.as_slice() {
                                                                            [only] if only.kind == AttachmentKind::FundusImage => {
                                                                                if let Some(att) = ui2::attach_stack::single(ui, only) {
                                                                                    pick_file = Some(att);
                                                                                }
                                                                            }
                                                                            _ if ui2::attach_stack::applies(&msg.attachments) => {
                                                                            if let Some(att) = ui2::attach_stack::show(ui, &msg.attachments) {
                                                                                pick_file = Some(att);
                                                                            }
                                                                            }
                                                                            _ => {
                                                                        // One chip per row: a wrapped row squeezes each chip to a one-letter column.
                                                                        ui.vertical(|ui| {
                                                                            for att in &msg.attachments {
                                                                                let icon = match att.kind {
                                                                                    AttachmentKind::FundusImage => "🖼",
                                                                                    AttachmentKind::PdfReport => "📄",
                                                                                    AttachmentKind::Dicom => "🏥",
                                                                                };
                                                                                let badge = Frame::new()
                                                                                    .fill(theme::INNER_TILE())
                                                                                    .stroke(Stroke::new(1.0, theme::BORDER_NEUTRAL()))
                                                                                    .corner_radius(CornerRadius::same(6))
                                                                                    .inner_margin(Margin::symmetric(8, 4));
                                                                                let badge_out = badge.show(ui, |ui| {
                                                                                    ui.set_max_width(ui.available_width().min(380.0));
                                                                                    let detail = if let Some(pgs) = att.page_count {
                                                                                        format!("{} ({} pgs)", att.display_name, pgs)
                                                                                    } else {
                                                                                        format!("{} ({} KB)", att.display_name, att.size_bytes / 1024)
                                                                                    };
                                                                                    ui.add(
                                                                                        egui::Label::new(
                                                                                            RichText::new(format!("{} {}", icon, detail))
                                                                                                .size(11.0)
                                                                                                .color(theme::TEXT_PRIMARY()),
                                                                                        )
                                                                                        .truncate(),
                                                                                    );
                                                                                });
                                                                                if att.kind != AttachmentKind::Dicom
                                                                                    && ui
                                                                                        .interact(badge_out.response.rect, ui.id().with(&att.id), egui::Sense::click())
                                                                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                                                        .clicked()
                                                                                {
                                                                                    pick_file = Some(att.clone());
                                                                                }
                                                                            }
                                                                        });
                                                                        }
}
                                                                    }

                                                                    ui.add_space(8.0);

                                                                    // Long message handling with collapsible preview
                                                                    let is_long = msg.text.lines().count() > 4
                                                                        || msg.text.len() > 180;
                                                                    if is_long && !msg.is_expanded {
                                                                        let preview: String =
                                                                            msg.text.chars().take(150).collect();
                                                                        ui.label(
                                                                            RichText::new(format!("{}...", preview))
                                                                                .size(13.5)
                                                                                .color(theme::TEXT_PRIMARY()),
                                                                        );
                                                                        ui.add_space(4.0);
                                                                        let toggle_btn = ui.add(
                                                                            Button::new(
                                                                                RichText::new("Show more ▾")
                                                                                    .size(12.0)
                                                                                    .color(theme::ACCENT_BLUE()),
                                                                            )
                                                                            .frame(false),
                                                                        );
                                                                        if toggle_btn.clicked() {
                                                                            toggle_expand_idx = Some(idx);
                                                                        }
                                                                    } else {
                                                                        ui.label(
                                                                            RichText::new(&msg.text)
                                                                                .size(13.5)
                                                                                .color(theme::TEXT_PRIMARY()),
                                                                        );
                                                                        if is_long {
                                                                            ui.add_space(4.0);
                                                                            let toggle_btn = ui.add(
                                                                                Button::new(
                                                                                    RichText::new("Show less ▴")
                                                                                        .size(12.0)
                                                                                        .color(theme::ACCENT_BLUE()),
                                                                                )
                                                                                .frame(false),
                                                                            );
                                                                            if toggle_btn.clicked() {
                                                                                toggle_expand_idx = Some(idx);
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            });
                                                        },
                                                    );
                                                });
                                                ui.add_space(22.0); // Clean 22px spacing between messages
                                            }
                                            MessageSender::Assistant => {
                                                // Assistant Message: NO ENCLOSING BOX (Directly on canvas)
                                                ui.horizontal(|ui| {
                                                    ui.with_layout(
                                                        Layout::left_to_right(Align::Min),
                                                        |ui| {
                                                            ui.vertical(|ui| {
                                                                ui.set_max_width(feed_width);

                                                                // Header: Title + Status + Timestamp
                                                                ui.horizontal(|ui| {
                                                                    ui::web_cards::avatar(ui, 40.0);
                                                                    ui.add_space(4.0);
                                                                    ui.label(
                                                                        RichText::new(match &msg.provider {
                                                                            Some(p) => format!("Chat assistant \u{b7} {p}"),
                                                                            None => "RetinaCare".to_string(),
                                                                        })
                                                                        .size(13.0)
                                                                        .strong()
                                                                        .color(theme::TEXT_PRIMARY()),
                                                                    );
                                                                    ui.add_space(8.0);
                                                                    match msg.status {
                                                                        ChatMessageStatus::Streaming => {
                                                                            ui.label(
                                                                                RichText::new("[Streaming...]")
                                                                                    .size(11.0)
                                                                                    .color(Color32::from_rgb(90, 175, 255)),
                                                                            );
                                                                        }
                                                                        ChatMessageStatus::Failed => {
                                                                            ui.label(
                                                                                RichText::new("[Failed]")
                                                                                    .size(11.0)
                                                                                    .color(Color32::from_rgb(248, 113, 113)),
                                                                            );
                                                                        }
                                                                        ChatMessageStatus::Cancelled => {
                                                                            ui.label(
                                                                                RichText::new("[Stopped]")
                                                                                    .size(11.0)
                                                                                    .color(Color32::from_rgb(217, 119, 6)),
                                                                            );
                                                                        }
                                                                        ChatMessageStatus::Queued => {
                                                                            ui.label(
                                                                                RichText::new("[Queued]")
                                                                                    .size(11.0)
                                                                                    .color(theme::TEXT_MUTED()),
                                                                            );
                                                                        }
                                                                        ChatMessageStatus::Completed => {}
                                                                    }
                                                                    ui.add_space(8.0);
                                                                    ui.label(
                                                                        RichText::new(&msg.timestamp)
                                                                            .size(11.0)
                                                                            .color(theme::TEXT_MUTED()),
                                                                    );
                                                                });

                                                                ui.add_space(10.0);

                                                                // Error Surface for Failed status
                                                                if msg.status == ChatMessageStatus::Failed {
                                                                    let err_box = Frame::new()
                                                                        .fill(Color32::from_rgb(42, 24, 24))
                                                                        .stroke(Stroke::new(1.0, Color32::from_rgb(110, 40, 40)))
                                                                        .corner_radius(CornerRadius::same(8))
                                                                        .inner_margin(Margin::symmetric(14, 10));
                                                                    err_box.show(ui, |ui| {
                                                                        ui.horizontal_wrapped(|ui| {
                                                                            ui.label(RichText::new("⚠️").size(14.0));
                                                                            ui.add_space(6.0);
                                                                            let err_msg = msg
                                                                                .error_detail
                                                                                .as_deref()
                                                                                .unwrap_or("AI service is not connected. Connect Freebuff backend to begin clinical consultation.");
                                                                            ui.label(
                                                                                RichText::new(err_msg)
                                                                                    .size(12.5)
                                                                                    .color(Color32::from_rgb(252, 165, 165)),
                                                                            );
                                                                        });
                                                                    });
                                                                    ui.add_space(8.0);
                                                                }

                                                                if let Some(web) = &msg.web {
                                                                    ui::web_cards::web_status(ui, web, idx);
                                                                } else if msg.status == ChatMessageStatus::Streaming && msg.text.is_empty() {
                                                                    ui.horizontal(|ui| {
                                                                        ui::web_cards::orb(ui, 22.0);
                                                                        ui.label(RichText::new("Thinking").size(12.5).color(Color32::from_rgb(109, 209, 135)));
                                                                    });
                                                                    ui.add_space(6.0);
                                                                }

                                                                let sites: &[domain::WebSite] = msg.web.as_ref().map_or(&[], |w| w.sites.as_slice());
                                                                if let Some(n) = ui::markdown::render(ui, &msg.text, chat_style, sites) {
                                                                    pick_site = sites.get(n).cloned();
                                                                }

                                                                // Streaming blinking cursor indicator
                                                                if msg.status == ChatMessageStatus::Streaming {
                                                                    ui.horizontal(|ui| {
                                                                        ui.label(
                                                                            RichText::new("▮")
                                                                                .size(14.0)
                                                                                .color(Color32::from_rgb(90, 175, 255)),
                                                                        );
                                                                    });
                                                                } else if msg.status == ChatMessageStatus::Cancelled {
                                                                    ui.label(
                                                                        RichText::new("*[Generation stopped by clinician]*")
                                                                            .size(12.0)
                                                                            .color(theme::TEXT_MUTED()),
                                                                    );
                                                                }

                                                                for src in &msg.sources {
                                                                    let label = format!("{} \u{b7} p. {}", ui::explain::ellipsize(&src.doc, 28), src.page);
                                                                    let chip = ui.add(
                                                                        Button::new(RichText::new(label).size(11.5).color(theme::ACCENT_BLUE()))
                                                                            .fill(theme::ACCENT_SOFT())
                                                                            .stroke(Stroke::new(1.0, theme::BORDER_NEUTRAL()))
                                                                            .corner_radius(CornerRadius::same(6)),
                                                                    );
                                                                    if chip.on_hover_text("Open the retrieved passage").clicked() {
                                                                        open_source = Some(src.clone());
                                                                    }
                                                                }
                                                                if let Some(note) = &msg.source_note {
                                                                    ui.label(RichText::new(note).size(12.0).color(theme::TEXT_MUTED()));
                                                                }

                                                                // Action toolbar below assistant message
                                                                ui.add_space(8.0);
                                                                ui.horizontal(|ui| {
                                                                    if !msg.text.is_empty() {
                                                                        let copy_btn = ui.add(
                                                                            Button::new(
                                                                                RichText::new("📋 Copy")
                                                                                    .size(11.5)
                                                                                    .color(theme::TEXT_SECONDARY()),
                                                                            )
                                                                            .fill(theme::CONTROL_RAISED())
                                                                            .stroke(Stroke::new(1.0, theme::BORDER_NEUTRAL()))
                                                                            .corner_radius(CornerRadius::same(6)),
                                                                        );
                                                                        if copy_btn.clicked() {
                                                                            ui.ctx().copy_text(msg.text.clone());
                                                                        }
                                                                    }

                                                                    if msg.status == ChatMessageStatus::Failed
                                                                        || msg.status == ChatMessageStatus::Cancelled
                                                                    {
                                                                        let retry_btn = ui.add(
                                                                            Button::new(
                                                                                RichText::new("🔄 Retry")
                                                                                    .size(11.5)
                                                                                    .color(Color32::from_rgb(120, 200, 255)),
                                                                            )
                                                                            .fill(theme::CONTROL_RAISED())
                                                                            .stroke(Stroke::new(1.0, theme::BORDER_NEUTRAL()))
                                                                            .corner_radius(CornerRadius::same(6)),
                                                                        );
                                                                        if retry_btn.clicked() {
                                                                            retry_trigger_idx = Some(idx);
                                                                        }
                                                                    }

                                                                    if msg.status == ChatMessageStatus::Streaming {
                                                                        let stop_btn = ui.add(
                                                                            Button::new(
                                                                                RichText::new("⏹ Stop")
                                                                                    .size(11.5)
                                                                                    .color(Color32::from_rgb(255, 160, 160)),
                                                                            )
                                                                            .fill(Color32::from_rgb(60, 24, 24))
                                                                            .stroke(Stroke::new(1.0, Color32::from_rgb(140, 50, 50)))
                                                                            .corner_radius(CornerRadius::same(6)),
                                                                        );
                                                                        if stop_btn.clicked() {
                                                                            cancel_trigger_req = msg.request_id.clone();
                                                                        }
                                                                    }
                                                                });
                                                            });
                                                        },
                                                    );
                                                });
                                                ui.add_space(26.0); // Generous vertical spacing after assistant message
                                            }
                                            MessageSender::SystemNotice => {}
                                        }
                                    }

                                    if card_link == Some(ui::chat_cards::CardLink::Save) {
                                        self.open_save_dialog();
                                        card_link = None;
                                    }
                                    if let Some(ui::chat_cards::CardLink::Assign(exam)) = card_link {
                                        self.open_assign_dialog(exam);
                                        card_link = None;
                                    }
                                    if let Some(ui::chat_cards::CardLink::Report(patient, exam)) = card_link {
                                        self.open_report(&ctx, patient, exam);
                                        card_link = None;
                                    }
                                    if let Some(link) = card_link {
                                        self.active_page = match link {
                                            ui::chat_cards::CardLink::Report(..) => self.active_page,
                                            ui::chat_cards::CardLink::Lesion => AppPage::LesionAnalysis,
                                            ui::chat_cards::CardLink::Vessel => AppPage::VesselAnalysis,
                                            ui::chat_cards::CardLink::Explain => AppPage::Explainability,
                                            ui::chat_cards::CardLink::Findings => AppPage::ResearchFindings,
                                            ui::chat_cards::CardLink::Save
                                            | ui::chat_cards::CardLink::Assign(_) => self.active_page,
                                        };
                                    }
                                    if let Some(site) = pick_site.take() {
                                        match self.open_pref {
                                            Some(true) => self.side = Some(SidePanel::Source(site)),
                                            Some(false) => ctx.open_url(egui::OpenUrl::new_tab(&site.url)),
                                            None => self.pending_site = Some(site),
                                        }
                                    }
                                    if let Some(att) = pick_file.take() {
                                        self.side = Some(SidePanel::File(att));
                                    }
                                    self.open_choice_ui(&ctx);
                                    if open_source.is_some() {
                                        self.source_view = open_source;
                                    }
                                    if let Some(t_idx) = toggle_expand_idx {
                                        if let Some(msg) = self.chat_messages.get_mut(t_idx) {
                                            msg.is_expanded = !msg.is_expanded;
                                        }
                                    }

                                    if let Some(r_idx) = retry_trigger_idx {
                                        let prev_user = self.chat_messages[..r_idx]
                                            .iter()
                                            .rposition(|m| m.sender == MessageSender::User);

                                        if let Some(u_idx) = prev_user {
                                            let user_msg = self.chat_messages[u_idx].clone();
                                            let req_id = format!("req_retry_{}", self.chat_messages.len() + 1);
                                            // A stored message lists only that attachments existed; the paper must be sent again.
                                            let paper_lost = user_msg.attachments.is_empty() && user_msg.stored_attachments > 0;
                                            if let Some(asst_msg) = self.chat_messages.get_mut(r_idx) {
                                                asst_msg.text.clear();
                                                asst_msg.sources.clear();
                                                asst_msg.source_note = None;
                                                asst_msg.error_detail = None;
                                                if paper_lost {
                                                    asst_msg.fail("The paper attached to that question is no longer open. Attach it again and resend.");
                                                } else {
                                                    asst_msg.status = ChatMessageStatus::Queued;
                                                    asst_msg.request_id = Some(req_id.clone());
                                                }
                                            }
                                            if !paper_lost {
                                                self.active_request_id = Some(req_id.clone());
                                                self.composer.is_busy = true;

                                                let chat_req = ChatRequest {
                                                    request_id: req_id,
                                                    session_id: self
                                                        .active_session_id
                                                        .clone()
                                                        .unwrap_or_else(|| "sess_default".to_string()),
                                                    prompt: user_msg.text,
                                                    attachments: user_msg.attachments,
                                                    model_identifier: self.composer.model_label.clone(),
                                                    history: self.chat_history_before(u_idx),
                                                    results_context: self.chat_results_context(true),
                                                };
                                                let _ = self.chat_service.submit(chat_req, self.chat_event_tx.clone());
                                            }
                                        }
                                    }

                                    if let Some(req_id) = cancel_trigger_req {
                                        let _ = self.chat_event_tx.send(ChatEvent::Cancelled { request_id: req_id });
                                    }
                                });
                            });
                    }
                }

                let mut close_source = false;
                if let Some(src) = &self.source_view {
                    egui::Window::new(format!("{} \u{b7} PDF page {}", src.doc, src.page))
                        .collapsible(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .default_width(520.0)
                        .show(&ctx, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Retrieved text from page {} of the attached PDF (document id {}). Page rendering is not available; this is the extracted text, and a printed page label may differ.",
                                    src.page, src.doc_id
                                ))
                                .size(12.0)
                                .color(theme::TEXT_MUTED()),
                            );
                            ui.separator();
                            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                ui.label(RichText::new(&src.excerpt).size(13.0));
                            });
                            if ui.button("Close").clicked() {
                                close_source = true;
                            }
                        });
                }
                if close_source {
                    self.source_view = None;
                }
                if dock_t > 0.01 {
                    let aa = Rect::from_min_size(
                        Pos2::new(main_rect.max.x - 56.0, main_rect.min.y),
                        Vec2::new(56.0, 24.0),
                    );
                    let mut aa_ui = ui.new_child(egui::UiBuilder::new().max_rect(aa));
                    aa_ui.menu_button(RichText::new("Aa").size(13.0), |ui| {
                        for (font, name) in ui::markdown::ChatFont::ALL {
                            ui.selectable_value(&mut self.chat_style.font, font, name);
                        }
                        ui.add(
                            egui::Slider::new(&mut self.chat_style.size, 12.0..=20.0).text("Size"),
                        );
                    });
                }

                // Render Animated Floating Composer
                let comp_rect = Rect::from_min_size(
                    Pos2::new(main_rect.min.x, main_rect.min.y + comp_top),
                    Vec2::new(main_rect.width(), composer_h),
                );
                let mut composer_ui = ui.new_child(egui::UiBuilder::new().max_rect(comp_rect));

                let action = if v2 {
                    let pal = crate::ui2::theme::mode(&ctx).palette();
                    let mut a = crate::ui2::composer::show(
                        &mut self.composer,
                        &mut composer_ui,
                        &ctx,
                        pal,
                        dock_t > 0.5,
                    );
                    // Test hook: RETINACARE_AUTOPICK_IMAGE attaches once at start without a click.
                    static AUTO: std::sync::Once = std::sync::Once::new();
                    if std::env::var_os("RETINACARE_AUTOPICK_IMAGE").is_some() {
                        AUTO.call_once(|| a = ComposerAction::PickImage);
                    }
                    static AUTO_PDF: std::sync::Once = std::sync::Once::new();
                    if std::env::var_os("RETINACARE_AUTOPICK_PDF").is_some() {
                        AUTO_PDF.call_once(|| a = ComposerAction::PickPdf);
                    }
                    a
                } else {
                    self.composer.show(&mut composer_ui)
                };
                match action {
                    ComposerAction::None => {}
                    ComposerAction::PickImage => {
                        self.pick_and_attach_images();
                    }
                    ComposerAction::PickPdf => {
                        self.pick_and_attach_pdfs();
                    }
                    ComposerAction::OpenPlugins => {
                        self.active_page = AppPage::Plugins;
                    }
                    ComposerAction::ChooseProject => {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            println!("RetinaCare project workspace: {:?}", folder);
                        }
                    }
                    ComposerAction::Cancel => self.chat_stop(),
                    ComposerAction::Submit(prompt_text, attachments) => {
                        if attachments.iter().any(|a| a.kind == AttachmentKind::FundusImage) {
                            let thumbs = attachments
                                .iter()
                                .filter(|a| a.kind == AttachmentKind::FundusImage)
                                .take(2)
                                .map(|a| eye_thumb(&ctx, &a.source_path))
                                .collect();
                            self.eye_choice = Some(EyeChoice {
                                prompt: prompt_text,
                                files: attachments,
                                patient: self.active_patient_id,
                                thumbs,
                                first_is_os: true,
                                since: std::time::Instant::now(),
                            });
                        } else {
                            self.chat_submit(&ctx, prompt_text, attachments);
                        }
                    }
                }
            }
            AppPage::Assessment => {
                let pal = crate::ui2::theme::mode(&ctx).palette();
                let eye = format!("{:?}", self.exam_view.laterality);
                crate::ui2::assessment_page::show(
                    ui,
                    pal,
                    main_rect,
                    self.exam_view.cascade.outcome.as_ref(),
                    &self.exam_view.exam_id,
                    &self.exam_view.patient_code,
                    &eye,
                );
            }
            AppPage::ReportDoc
                if self.active_patient_id.is_none() || self.active_examination_id.is_none() =>
            {
                let pal = crate::ui2::theme::mode(&ctx).palette();
                ui.painter().text(
                    main_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No examination selected.\nOpen a patient and choose an examination to see its report.",
                    egui::FontId::proportional(16.0),
                    pal.text_muted,
                );
            }
            AppPage::ReportDoc => {
                // Load the saved explanation of this exam first, so the report shows it after a restart.
                self.explain_sync_context(&ctx);
                if self.report_doc.is_none() {
                    self.report_doc = Some(self.build_pdf_document());
                }
                let pal = crate::ui2::theme::mode(&ctx).palette();
                let doc = self.report_doc.as_ref().expect("built above");
                let exam = self.exam_view.exam_id.clone();
                let code = self.exam_view.patient_code.clone();
                if crate::ui2::report_page::show(
                    ui,
                    &ctx,
                    pal,
                    main_rect,
                    doc,
                    (&exam, &code),
                    &mut self.report_view,
                ) == crate::ui2::report_page::ReportAction::ExportPdf
                {
                    self.submit_pdf_export();
                }
            }
            AppPage::RetinalImaging => {
                use crate::ui2::retinal_imaging::{self as ri, RetinalImagingAction as A};
                let pal = crate::ui2::theme::mode(&ctx).palette();
                let cx = ri::RetinalImagingContext {
                    exam: &self.exam_view,
                    original: self.mode_textures[0].as_ref(),
                    file_name: self.uploaded_filename.as_deref(),
                };
                match ri::show(ui, pal, main_rect, &mut self.retinal_imaging, &cx) {
                    A::OpenReview => self.active_page = AppPage::Report,
                    A::OpenReport => self.active_page = AppPage::ReportDoc,
                    A::OpenBilateral => self.active_page = AppPage::Bilateral,
                    A::None => {}
                }
            }
            AppPage::Bilateral => self.render_bilateral(ui, main_rect, &ctx),
            AppPage::Report => {
                let pdf_readiness = self.pdf_export_service.readiness();
                let audit_readiness = self.audit_ledger_service.readiness();
                let action = if crate::ui2::theme::enabled() {
                    let pal = crate::ui2::theme::mode(&ctx).palette();
                    let out = crate::ui2::review_page::show(
                        ui,
                        pal,
                        main_rect,
                        &mut self.exam_view,
                        self.mode_textures[0].as_ref(),
                        self.uploaded_filename.as_deref(),
                        &mut self.review_view,
                    );
                    if out.open_report {
                        self.active_page = AppPage::ReportDoc;
                    }
                    out.action
                } else {
                    ui::viewer::render_retinal_workspace(
                        ui,
                        &ctx,
                        main_rect,
                        &mut self.exam_view,
                        &self.mode_textures,
                        self.uploaded_filename.as_deref(),
                        self.uploaded_dimensions,
                        &self.backend_readiness,
                        self.stage1_result.as_ref(),
                        self.stage1_error.as_deref(),
                        &pdf_readiness,
                        &audit_readiness,
                        &self.export_state,
                        &self.cached_audit_events,
                        self.session_export_status.as_deref(),
                    )
                };

                match action {
                    ViewerAction::RunStage1Inference => self.start_analysis(),
                    ViewerAction::CancelInference => self.cancel_analysis(),
                    ViewerAction::DismissResult => {
                        self.stage1_result = None;
                        self.stage1_error = None;
                        self.exam_view.cascade.error = None;
                    }
                    ViewerAction::UploadImageRequested => {
                        self.pick_and_load_image(&ctx);
                    }
                    ViewerAction::LateralityChanged(lat) => {
                        // Results belong to one eye: switching eyes drops them (and any running job).
                        if lat != self.exam_view.laterality {
                            self.reset_analysis_context();
                        }
                        self.exam_view.laterality = lat;
                    }
                    ViewerAction::SyncModeChanged(mode) => {
                        self.exam_view.sync_mode = mode;
                    }
                    ViewerAction::ExpandPanel(opt) => {
                        self.exam_view.expanded_panel = opt;
                    }
                    ViewerAction::ResetAll => {
                        self.viewer.reset();
                    }
                    ViewerAction::FitAll => {
                        self.viewer.zoom = 1.0;
                        self.viewer.pan = Vec2::ZERO;
                    }
                    ViewerAction::SyncZoomAndPan { zoom, pan } => {
                        self.viewer.zoom = zoom;
                        self.viewer.pan = Vec2::new(pan[0], pan[1]);
                    }
                    ViewerAction::ToggleReportViewMode => {
                        ctx.request_repaint();
                    }
                    ViewerAction::OpenExportPdfDialog => {
                        self.exam_view.export_dialog_open = true;
                        self.exam_view.audit_ledger_open = false;
                        ctx.request_repaint();
                    }
                    ViewerAction::OpenSessionExportDialog => {
                        let dest = std::env::temp_dir();
                        match crate::engine::export_scoped_patient_session(
                            &dest,
                            &self.exam_view,
                            self.stage1_result.as_ref(),
                        ) {
                            Ok(path) => {
                                self.session_export_status = Some(format!(
                                    "Plaintext Sensitive Export (Unencrypted JSON) saved to {}",
                                    path.display()
                                ));
                                let _ =
                                    self.audit_ledger_service
                                        .record_event(crate::engine::AuditEvent {
                                        event_id: format!(
                                            "evt_{}",
                                            std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_secs())
                                                .unwrap_or(0)
                                        ),
                                        timestamp: crate::engine::report::utc_now(),
                                        event_type:
                                            crate::engine::AuditEventType::SessionExportCompleted,
                                        examination_id: self.exam_view.exam_id.clone(),
                                        request_id: "req_sess_export".to_string(),
                                        operator: current_operator(),
                                        status: crate::engine::AuditEventStatus::Success,
                                        non_sensitive_summary: format!(
                                            "Scoped patient session dossier written to {}",
                                            path.display()
                                        ),
                                    });
                                self.cached_audit_events = self
                                    .audit_ledger_service
                                    .query_events(Some(&self.exam_view.exam_id))
                                    .unwrap_or_default();
                            }
                            Err(e) => {
                                self.session_export_status =
                                    Some(format!("Session export failed: {e}"));
                            }
                        }
                        ctx.request_repaint();
                    }
                    ViewerAction::ToggleAuditLedger => {
                        self.cached_audit_events = self
                            .audit_ledger_service
                            .query_events(Some(&self.exam_view.exam_id))
                            .unwrap_or_default();
                        ctx.request_repaint();
                    }
                    ViewerAction::SubmitPdfExport => self.submit_pdf_export(),
                    ViewerAction::SubmitSessionExport => {
                        let dest = std::env::temp_dir();
                        match crate::engine::export_scoped_patient_session(
                            &dest,
                            &self.exam_view,
                            self.stage1_result.as_ref(),
                        ) {
                            Ok(path) => {
                                self.session_export_status = Some(format!(
                                    "Plaintext Sensitive Export (Unencrypted JSON) saved to {}",
                                    path.display()
                                ));
                                let _ =
                                    self.audit_ledger_service
                                        .record_event(crate::engine::AuditEvent {
                                        event_id: format!(
                                            "evt_{}",
                                            std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_secs())
                                                .unwrap_or(0)
                                        ),
                                        timestamp: crate::engine::report::utc_now(),
                                        event_type:
                                            crate::engine::AuditEventType::SessionExportCompleted,
                                        examination_id: self.exam_view.exam_id.clone(),
                                        request_id: "req_sess_export".to_string(),
                                        operator: current_operator(),
                                        status: crate::engine::AuditEventStatus::Success,
                                        non_sensitive_summary: format!(
                                            "Scoped patient session dossier written to {}",
                                            path.display()
                                        ),
                                    });
                                self.cached_audit_events = self
                                    .audit_ledger_service
                                    .query_events(Some(&self.exam_view.exam_id))
                                    .unwrap_or_default();
                            }
                            Err(e) => {
                                self.session_export_status =
                                    Some(format!("Session export failed: {e}"));
                            }
                        }
                        ctx.request_repaint();
                    }
                    ViewerAction::CloseModals => {
                        self.exam_view.export_dialog_open = false;
                        self.exam_view.audit_ledger_open = false;
                        ctx.request_repaint();
                    }
                    ViewerAction::NavigateReportSubroute(subroute) => {
                        self.exam_view.current_subroute = subroute;
                        if subroute == crate::domain::ReportSubroute::AuditHistory {
                            self.cached_audit_events = self
                                .audit_ledger_service
                                .query_events(Some(&self.exam_view.exam_id))
                                .unwrap_or_default();
                        }
                        ctx.request_repaint();
                    }
                    ViewerAction::BackToReportWorkspace => {
                        self.exam_view.current_subroute = if self.exam_view.report_view_mode {
                            crate::domain::ReportSubroute::ReportOverview
                        } else {
                            crate::domain::ReportSubroute::QuadViewer
                        };
                        ctx.request_repaint();
                    }
                    ViewerAction::None => {}
                }
            }
            AppPage::PreviousChats => {
                let persisted = self.persistence_available();
                let strip = Rect::from_min_size(
                    main_rect.min,
                    Vec2::new(main_rect.width(), cloud_app::STRIP_HEIGHT),
                );
                let history_rect = Rect::from_min_max(
                    Pos2::new(main_rect.min.x, main_rect.min.y + cloud_app::STRIP_HEIGHT),
                    main_rect.max,
                );
                self.render_cloud_strip(ui, strip, &ctx);
                if crate::ui2::theme::enabled() {
                    let pal = crate::ui2::theme::mode(&ctx).palette();
                    if let Some(id) = crate::ui2::chats_page::show(
                        ui,
                        pal,
                        history_rect,
                        &mut self.chats_view,
                        &self.previous_sessions,
                        self.active_session_id.as_deref(),
                        persisted,
                    ) {
                        self.open_conversation(&ctx, &id, true);
                    }
                    if std::mem::take(&mut self.chats_view.delete_all) {
                        self.delete_all_chats(&ctx);
                    }
                } else {
                    let action = ui::render_history_workspace(
                        ui,
                        history_rect,
                        &mut self.history_view_state,
                        &self.previous_sessions,
                        self.active_session_id.as_deref(),
                        self.active_patient_id,
                        std::env::var("RETINACARE_DEV_FIXTURES").is_ok(),
                        persisted,
                    );
                    self.handle_history_action(action, &ctx);
                }
            }
            AppPage::Timeline => {
                let count = match self.audit_ledger_service.readiness() {
                    crate::engine::AuditLedgerReadiness::Ready { record_count } => {
                        Some(record_count)
                    }
                    crate::engine::AuditLedgerReadiness::Unavailable { .. } => None,
                };
                if count != self.all_audit_count {
                    self.all_audit_events = self
                        .audit_ledger_service
                        .query_events(None)
                        .unwrap_or_default();
                    self.all_audit_count = count;
                }
                let action = ui::render_timeline_workspace(
                    ui,
                    main_rect,
                    &mut self.timeline_view_state,
                    self.patient_service.as_ref(),
                    self.active_patient_id,
                    std::env::var("RETINACARE_DEV_FIXTURES").is_ok(),
                    self.patient_service.unlock_status()
                        == Some(crate::engine::store::UnlockStatus::Open),
                    &self.all_audit_events,
                );
                self.handle_timeline_action(action, &ctx);
            }
            AppPage::Patients => {
                let action = ui::render_patients_workspace(
                    ui,
                    main_rect,
                    &mut self.patients_view_state,
                    self.patient_service.as_ref(),
                );
                self.handle_patients_action(action, &ctx);
            }
            AppPage::Explainability => {
                self.render_explainability_page(ui, main_rect, &ctx);
            }
            AppPage::LesionAnalysis | AppPage::VesselAnalysis | AppPage::ResearchFindings => {
                let page = self.active_page;
                self.render_science_page(ui, main_rect, page, &ctx);
            }
            AppPage::Plugins => {
                // Modular Plugin subsystem rendered on identical #191919 workspace canvas
                if crate::ui2::theme::enabled() {
                    let pal = crate::ui2::theme::mode(&ctx).palette();
                    let cards = self.plugin_cards();
                    crate::ui2::plugins_page::show(ui, pal, main_rect, &cards);
                } else {
                    self.plugins_state.show(ui, main_rect);
                }
            }
        }
    }
}

impl RetinaCareApp {
    fn handle_patients_action(&mut self, action: ui::PatientsAction, ctx: &egui::Context) {
        match action {
            ui::PatientsAction::None => {}
            ui::PatientsAction::SelectPatient(opt_id) => {
                // Enforce strict cross-patient isolation:
                // Drop prior image textures, clear inference results, and invalidate active request IDs
                self.clear_loaded_image();
                self.active_request_id = None;
                self.reset_analysis_context();
                self.forget_exam_context();
                self.active_patient_id = opt_id;

                if let Some(id) = opt_id {
                    if let Ok(detail) = self.patient_service.get_patient(id) {
                        self.exam_view.patient_code = detail.patient_code.clone();
                        self.patients_view_state.select_patient(Some(&detail));
                    }
                } else {
                    self.patients_view_state.select_patient(None);
                }
                ctx.request_repaint();
            }
            ui::PatientsAction::SelectExamination(opt_exam) => {
                if opt_exam != self.active_examination_id {
                    self.clear_loaded_image();
                    self.reset_analysis_context();
                }
                self.active_examination_id = opt_exam;
                if let Some(exam_id) = opt_exam {
                    self.exam_view.exam_id = format!("EXAM-{exam_id:03}");
                }
                ctx.request_repaint();
            }
            ui::PatientsAction::OpenInReport {
                patient_id,
                exam_id,
            } => {
                self.load_patient_exam_into_workspace(patient_id, exam_id, true, ctx);
            }
            ui::PatientsAction::OpenInViewer {
                patient_id,
                exam_id,
            } => {
                self.load_patient_exam_into_workspace(patient_id, exam_id, false, ctx);
            }
            ui::PatientsAction::AttachImageToExam {
                exam_id: _,
                image_path: _,
            } => {
                ctx.request_repaint();
            }
            ui::PatientsAction::SavePatient { id, code, name } => {
                match self.patient_service.update_patient(id, &code, &name) {
                    Ok(()) => {
                        self.patients_view_state.edit_dirty = false;
                        self.patients_view_state.edit_status_msg =
                            Some(("Saved patient changes successfully.".to_string(), false));
                        self.exam_view.patient_code = code;
                    }
                    Err(e) => {
                        self.patients_view_state.edit_status_msg =
                            Some((format!("Save failed: {e}"), true));
                    }
                }
                ctx.request_repaint();
            }
            ui::PatientsAction::CreatePatient { code, name } => {
                match self.patient_service.add_patient(&code, &name) {
                    Ok(new_id) => {
                        self.patients_view_state.new_patient_modal_open = false;
                        if let Ok(detail) = self.patient_service.get_patient(new_id) {
                            self.patients_view_state.select_patient(Some(&detail));
                            self.active_patient_id = Some(new_id);
                            self.exam_view.patient_code = detail.patient_code;
                        }
                    }
                    Err(e) => {
                        self.patients_view_state.new_patient_error =
                            Some(format!("Registration failed: {e}"));
                    }
                }
                ctx.request_repaint();
            }
            ui::PatientsAction::NavigateSubroute(subroute) => {
                match subroute {
                    crate::domain::PatientSubroute::Directory => {
                        self.patients_view_state.current_subroute = subroute;
                    }
                    crate::domain::PatientSubroute::Profile { patient_id } => {
                        self.active_patient_id = Some(patient_id);
                        if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                            self.exam_view.patient_code = detail.patient_code.clone();
                            self.patients_view_state.select_patient(Some(&detail));
                        }
                        self.patients_view_state.current_subroute = subroute;
                    }
                    crate::domain::PatientSubroute::ExamDetail {
                        patient_id,
                        examination_id,
                    } => {
                        self.active_patient_id = Some(patient_id);
                        self.active_examination_id = Some(examination_id);
                        if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                            self.exam_view.patient_code = detail.patient_code.clone();
                            self.patients_view_state.select_patient(Some(&detail));
                        }
                        self.patients_view_state.current_subroute = subroute;
                    }
                }
                ctx.request_repaint();
            }
            ui::PatientsAction::PreviewPatient(opt_id) => {
                self.patients_view_state.preview_patient_id = opt_id;
                ctx.request_repaint();
            }
            ui::PatientsAction::CreateExamination(new_exam) => {
                match self.patient_service.add_examination(&new_exam) {
                    Ok(_) => {
                        self.patients_view_state.new_exam_modal_open = false;
                    }
                    Err(e) => {
                        self.patients_view_state.new_exam_error =
                            Some(format!("Failed to record examination: {e}"));
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    fn handle_history_action(&mut self, action: crate::domain::HistoryAction, ctx: &egui::Context) {
        match action {
            crate::domain::HistoryAction::None => {}
            crate::domain::HistoryAction::NavigateSubroute(subroute) => {
                self.history_view_state.current_subroute = subroute;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::SelectConversation(opt_id) => {
                if let Some(id) = &opt_id {
                    self.open_conversation(ctx, id, true);
                }
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::StartNewConsultation => {
                self.chat_messages.clear();
                self.active_session_id = None;
                self.reset_analysis_context();
                self.is_chat_docked = false;
                self.active_page = AppPage::Home;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::OpenInPatientProfile(pid) => {
                self.active_patient_id = Some(pid);
                if let Ok(detail) = self.patient_service.get_patient(pid) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.patients_view_state.current_subroute =
                    crate::domain::PatientSubroute::Profile { patient_id: pid };
                self.active_page = AppPage::Patients;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::OpenInRetinalExam {
                patient_id,
                exam_id,
            } => {
                self.active_patient_id = Some(patient_id);
                self.active_examination_id = Some(exam_id);
                if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.patients_view_state.current_subroute =
                    crate::domain::PatientSubroute::ExamDetail {
                        patient_id,
                        examination_id: exam_id,
                    };
                self.active_page = AppPage::Patients;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::OpenInTimeline { patient_id } => {
                self.active_patient_id = Some(patient_id);
                if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.active_page = AppPage::Timeline;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::OpenInQuad => {
                self.exam_view.current_subroute = crate::domain::ReportSubroute::QuadViewer;
                self.active_page = AppPage::Report;
                ctx.request_repaint();
            }
            crate::domain::HistoryAction::RetryQuery => {
                ctx.request_repaint();
            }
        }
    }

    fn handle_timeline_action(
        &mut self,
        action: crate::domain::TimelineAction,
        ctx: &egui::Context,
    ) {
        match action {
            crate::domain::TimelineAction::None => {}
            crate::domain::TimelineAction::NavigateSubroute(subroute) => {
                self.timeline_view_state.current_subroute = subroute;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::SelectEvent(opt_id) => {
                self.timeline_view_state.selected_event_id = opt_id;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::SelectPatient(opt_id) => {
                // Cross-patient isolation: clear selection and cached state
                if self.active_patient_id != opt_id {
                    self.clear_loaded_image();
                    self.reset_analysis_context();
                }
                self.active_patient_id = opt_id;
                self.timeline_view_state.selected_event_id = None;
                if let Some(id) = opt_id {
                    if let Ok(detail) = self.patient_service.get_patient(id) {
                        self.exam_view.patient_code = detail.patient_code.clone();
                        self.patients_view_state.select_patient(Some(&detail));
                    }
                } else {
                    self.patients_view_state.select_patient(None);
                }
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::SetLateralityFilter(opt_lat) => {
                self.timeline_view_state.filter.laterality = opt_lat;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::OpenInQuad { examination_id } => {
                if self.active_examination_id != Some(examination_id) {
                    self.clear_loaded_image();
                    self.reset_analysis_context();
                    if let Some(patient_id) = self.active_patient_id {
                        self.load_exam_media(ctx, patient_id, examination_id);
                    }
                    self.restore_saved_analysis(ctx, examination_id);
                }
                self.active_examination_id = Some(examination_id);
                self.exam_view.exam_id = format!("EXAM-{examination_id:03}");
                self.exam_view.current_subroute = crate::domain::ReportSubroute::QuadViewer;
                self.active_page = AppPage::Report;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::OpenInReport {
                patient_id,
                examination_id,
            } => {
                if self.active_examination_id != Some(examination_id)
                    || self.active_patient_id != Some(patient_id)
                {
                    self.clear_loaded_image();
                    self.reset_analysis_context();
                    self.load_exam_media(ctx, patient_id, examination_id);
                    self.restore_saved_analysis(ctx, examination_id);
                }
                self.active_patient_id = Some(patient_id);
                self.active_examination_id = Some(examination_id);
                if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.exam_view.exam_id = format!("EXAM-{examination_id:03}");
                self.exam_view.current_subroute = crate::domain::ReportSubroute::ReportOverview;
                self.active_page = AppPage::Report;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::OpenPatientProfile { patient_id } => {
                self.active_patient_id = Some(patient_id);
                if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.patients_view_state.current_subroute =
                    crate::domain::PatientSubroute::Profile { patient_id };
                self.active_page = AppPage::Patients;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::OpenExamDetail {
                patient_id,
                examination_id,
            } => {
                self.active_patient_id = Some(patient_id);
                self.active_examination_id = Some(examination_id);
                if let Ok(detail) = self.patient_service.get_patient(patient_id) {
                    self.exam_view.patient_code = detail.patient_code.clone();
                    self.patients_view_state.select_patient(Some(&detail));
                }
                self.patients_view_state.current_subroute =
                    crate::domain::PatientSubroute::ExamDetail {
                        patient_id,
                        examination_id,
                    };
                self.active_page = AppPage::Patients;
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::ZoomIn => {
                self.timeline_view_state.zoom_level =
                    (self.timeline_view_state.zoom_level * 1.25).min(4.0);
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::ZoomOut => {
                self.timeline_view_state.zoom_level =
                    (self.timeline_view_state.zoom_level / 1.25).max(0.5);
                ctx.request_repaint();
            }
            crate::domain::TimelineAction::ResetZoom => {
                self.timeline_view_state.zoom_level = 1.0;
                self.timeline_view_state.pan_offset = 0.0;
                ctx.request_repaint();
            }
        }
    }

    fn load_patient_exam_into_workspace(
        &mut self,
        patient_id: i64,
        exam_id: i64,
        report_mode: bool,
        ctx: &egui::Context,
    ) {
        // Enforce strict cross-patient isolation:
        // Drop prior image textures, clear inference results, and invalidate active request IDs
        self.clear_loaded_image();
        self.active_request_id = None;
        self.reset_analysis_context();

        self.active_patient_id = Some(patient_id);
        self.active_examination_id = Some(exam_id);

        if let Ok(detail) = self.patient_service.get_patient(patient_id) {
            self.exam_view.patient_code = detail.patient_code.clone();
            self.patients_view_state.select_patient(Some(&detail));
        }

        self.exam_view.exam_id = format!("EXAM-{exam_id:03}");

        self.load_exam_media(ctx, patient_id, exam_id);

        // After the image load (which resets analysis state): show the saved summary, if any.
        self.restore_saved_analysis(ctx, exam_id);
        self.exam_view.report_view_mode = report_mode;
        self.exam_view.current_subroute = if report_mode {
            crate::domain::ReportSubroute::ReportOverview
        } else {
            crate::domain::ReportSubroute::QuadViewer
        };
        self.active_page = AppPage::Report;
        ctx.request_repaint();
    }
}

/// The RetinaCare icon as window RGBA, decoded from the bundled ICO. None if it cannot be decoded.
fn app_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../../icon/RetinaCare.ico")).ok()?;
    // 128 px: a 256 px icon is about 262 KB, over the X11 default request limit, and the server drops it.
    let rgba = img
        .resize(128, 128, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    // Same default as packaging/retinacare-launch: models load on CUDA device 0 first; the CPU is used
    // only when the GPU cannot load them (the loaded model reports the device it really runs on).
    // Set RETINACARE_PROVIDER=cuda for a hard GPU-only mode, cpu to force the CPU. Runs before any thread starts.
    if std::env::var_os("RETINACARE_PROVIDER").is_none() {
        std::env::set_var("RETINACARE_PROVIDER", "auto");
    }
    let icon_bytes = include_bytes!("../../icon/RetinaCare.ico");
    let icon_data = if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        }
    } else {
        egui::IconData::default()
    };

    let (win_w, win_h) = if let Ok(size_str) = std::env::var("RETINACARE_WINDOW_SIZE") {
        let parts: Vec<&str> = size_str.split('x').collect();
        if parts.len() == 2 {
            (
                parts[0].parse::<f32>().unwrap_or(1280.0),
                parts[1].parse::<f32>().unwrap_or(820.0),
            )
        } else {
            (1280.0, 820.0)
        }
    } else {
        (1280.0, 820.0)
    };

    let renderer = if let Ok(r) = std::env::var("RETINACARE_RENDERER") {
        if r.to_lowercase() == "wgpu" {
            eframe::Renderer::Wgpu
        } else {
            eframe::Renderer::Glow
        }
    } else {
        eframe::Renderer::Glow
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("retinacare") // Binds to retinacare.desktop for Ubuntu dock icon
            .with_title("RetinaCare AI Screening Suite")
            .with_inner_size([win_w, win_h])
            .with_min_inner_size([480.0, 360.0])
            .with_decorations(false) // No white OS title bar
            .with_transparent(true)
            .with_icon(Arc::new(icon_data)),
        renderer,
        ..Default::default()
    };

    println!("===============================================================");
    println!("  RetinaCare AI Screening Suite &mdash; 100% Native Rust Software");
    println!("  Platform: Ubuntu Linux x86_64 Desktop Executable Binary");
    println!("  Zero TypeScript, Zero TSX &mdash; Native OpenGL/Wayland/X11 GUI");
    println!("===============================================================");

    eframe::run_native(
        "RetinaCare AI Screening Suite",
        native_options,
        Box::new(|cc| Ok(Box::new(RetinaCareApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ArtifactLoadStatus, EyeLaterality, SyncMode, ViewPanelId};
    use std::path::PathBuf;

    #[test]
    fn test_app_initial_state() {
        let app = RetinaCareApp::default();
        assert!(app.sidebar_open);
        assert_eq!(app.active_page, AppPage::Home);
        assert_eq!(app.active_mode, 0);
        assert_eq!(app.viewer.zoom, 1.0);
        assert_eq!(app.viewer.pan, Vec2::ZERO);
        assert_eq!(app.composer.prompt, "");
        assert_eq!(app.composer.model_label, "RetinaCare Research Analysis");
        assert!(app.mode_textures[0].is_none());
        assert!(app.mode_textures[1].is_none());
        assert!(app.mode_textures[2].is_none());
        assert!(app.mode_textures[3].is_none());
        assert!(app.uploaded_filename.is_none());
        assert!(app.chat_messages.is_empty());
        assert!(!app.is_chat_docked);
        assert_eq!(app.hierarchi_manifest.stages.len(), 4);
    }

    #[test]
    fn test_sidebar_toggle_state() {
        let mut app = RetinaCareApp::default();
        assert!(app.sidebar_open);
        app.sidebar_open = !app.sidebar_open;
        assert!(!app.sidebar_open);
        app.sidebar_open = !app.sidebar_open;
        assert!(app.sidebar_open);
    }

    #[test]
    fn test_modes_count() {
        assert_eq!(MODES.len(), 4);
        assert_eq!(MODES[0].0, "Original Image");
    }

    #[test]
    fn test_bullet_strip_prefix() {
        let line = "• Multimodal Ingestion Pipeline";
        let content = line
            .strip_prefix("• ")
            .expect("Should safely strip UTF-8 bullet prefix");
        assert_eq!(content, "Multimodal Ingestion Pipeline");
    }

    #[test]
    fn test_user_bubble_adaptive_width() {
        let text_short = "hi";
        let longest_line_len = text_short
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let char_px = 8.5;
        let text_content_w = (longest_line_len as f32 * char_px).clamp(24.0, 800.0 * 0.72);
        let min_bubble_w = 160.0_f32;
        let bubble_width = (text_content_w + 48.0).max(min_bubble_w).min(800.0 * 0.78);
        assert_eq!(bubble_width, 160.0); // Properly sizes short user prompt
    }

    #[test]
    fn test_hierarchi_manifest_audit() {
        let manifest = HierarchiRetinaManifest::load_audit();
        assert_eq!(manifest.version, "v1.0");
        assert_eq!(manifest.stages.len(), 4);
        use crate::engine::manifest::{CheckpointStatus, ModelFormat};
        for stage in &manifest.stages {
            assert_eq!(stage.status, CheckpointStatus::VerifiedActive);
            assert_eq!(stage.format, ModelFormat::Onnx);
        }
        assert!(manifest.stages[3].notes.contains("FOLD 0 ONLY unless"));
    }

    #[test]
    fn test_viewer_reset() {
        let mut viewer = ImageViewerState {
            zoom: 2.5,
            pan: Vec2::new(100.0, -50.0),
        };
        viewer.reset();
        assert_eq!(viewer.zoom, 1.0);
        assert_eq!(viewer.pan, Vec2::ZERO);
    }

    #[test]
    fn test_embedded_icon() {
        let icon_bytes = include_bytes!("../../icon/RetinaCare.ico");
        let img = image::load_from_memory(icon_bytes).expect("Valid icon file");
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);
    }

    #[test]
    fn test_default_sessions_loaded() {
        let app = RetinaCareApp::default();
        assert!(!app.previous_sessions.is_empty());
        assert_eq!(app.previous_sessions.len(), 5);
        assert_eq!(
            app.previous_sessions[0].title,
            "OD Screening - Microaneurysms"
        );
    }

    #[test]
    fn test_session_switching_and_docking() {
        let mut app = RetinaCareApp::default();
        let session = &app.previous_sessions[1]; // Bilateral Macular Assessment
        app.chat_messages = session.messages.clone();
        app.active_session_id = Some(session.id.clone());
        app.is_chat_docked = true;
        app.active_page = AppPage::Home;

        assert_eq!(app.active_page, AppPage::Home);
        assert!(app.is_chat_docked);
        assert_eq!(app.chat_messages.len(), 2);
        assert_eq!(app.active_session_id.as_deref(), Some("sess_02"));
    }

    #[test]
    fn test_new_consultation_reset() {
        let mut app = RetinaCareApp::default();
        // Simulate an ongoing chat
        app.chat_messages = app.previous_sessions[0].messages.clone();
        app.active_session_id = Some("sess_01".to_string());
        app.is_chat_docked = true;

        // Perform "+ New Consultation" reset
        app.chat_messages.clear();
        app.active_session_id = None;
        app.is_chat_docked = false;
        app.active_page = AppPage::Home;

        assert!(app.chat_messages.is_empty());
        assert!(app.active_session_id.is_none());
        assert!(!app.is_chat_docked);
        assert_eq!(app.active_page, AppPage::Home);
    }

    #[test]
    fn test_plugins_navigation() {
        let mut app = RetinaCareApp::default();
        assert_eq!(app.active_page, AppPage::Home);
        app.active_page = AppPage::Plugins;
        assert_eq!(app.active_page, AppPage::Plugins);
        assert_eq!(app.plugins_state.registry.plugins.len(), 12);
    }

    #[test]
    fn test_empty_message_validation() {
        let composer = HomeComposer {
            prompt: "   ".to_string(),
            ..Default::default()
        };
        assert!(!composer.has_valid_payload());
    }

    #[test]
    fn test_attachment_only_submission() {
        let mut composer = HomeComposer {
            prompt: "".to_string(),
            ..Default::default()
        };
        composer.attachments.push(LocalAttachment {
            id: "att_test".to_string(),
            display_name: "od_scan.png".to_string(),
            kind: AttachmentKind::FundusImage,
            source_path: std::path::PathBuf::from("/tmp/od_scan.png"),
            size_bytes: 2048,
            page_count: None,
            extracted_text: None,
            sha256_hash: Some("112233445566".to_string()),
        });
        assert!(composer.has_valid_payload());
    }

    #[test]
    fn test_multiline_content_preservation() {
        let multiline =
            "Line 1: Retinal OD assessment\nLine 2: 12 microaneurysms\nLine 3: Caliber 2:3";
        let msg = ChatMessage::new_user("usr_multi", multiline, "11:00 AM", vec![]);
        assert_eq!(msg.text, multiline);
        assert_eq!(msg.text.lines().count(), 3);
    }

    #[test]
    fn test_streaming_delta_accumulation_flow() {
        let mut msg = ChatMessage::new_assistant(
            "asst_stream_1",
            "",
            "11:01 AM",
            ChatMessageStatus::Queued,
            Some("req_stream_1".to_string()),
        );
        assert_eq!(msg.status, ChatMessageStatus::Queued);

        msg.append_delta("Segmenting retinal vasculature... ");
        assert_eq!(msg.status, ChatMessageStatus::Streaming);
        assert_eq!(msg.text, "Segmenting retinal vasculature... ");

        msg.append_delta("No neovascularization detected.");
        assert_eq!(msg.status, ChatMessageStatus::Streaming);
        assert_eq!(
            msg.text,
            "Segmenting retinal vasculature... No neovascularization detected."
        );

        msg.complete();
        assert_eq!(msg.status, ChatMessageStatus::Completed);
    }

    #[test]
    fn test_chat_cancellation_flow() {
        let mut msg = ChatMessage::new_assistant(
            "asst_cancel_1",
            "Partial inference generated before user cancelled...",
            "11:02 AM",
            ChatMessageStatus::Streaming,
            Some("req_cancel_1".to_string()),
        );
        msg.cancel();
        assert_eq!(msg.status, ChatMessageStatus::Cancelled);
        assert_eq!(
            msg.text,
            "Partial inference generated before user cancelled..."
        );
    }

    #[test]
    fn test_unavailable_backend_event_flow() {
        let mut app = RetinaCareApp::default();
        let prompt = "Analyze hemorrhage pattern".to_string();
        let req_id = "req_unavail_test".to_string();

        app.chat_messages.push(ChatMessage::new_user(
            "usr_test",
            prompt.clone(),
            "11:03 AM",
            vec![],
        ));
        app.chat_messages.push(ChatMessage::new_assistant(
            "asst_test",
            "",
            "11:03 AM",
            ChatMessageStatus::Queued,
            Some(req_id.clone()),
        ));

        let chat_req = ChatRequest {
            request_id: req_id.clone(),
            session_id: "sess_test".to_string(),
            prompt,
            attachments: vec![],
            model_identifier: "HierarchiRetina (Awaiting Backend)".to_string(),
            history: Vec::new(),
            results_context: None,
        };

        UnavailableChatService
            .submit(chat_req, app.chat_event_tx.clone())
            .expect("Submit succeeds");

        // Drain the events directly into app state
        if let Some(rx) = &app.chat_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ChatEvent::Started { request_id, .. } => {
                        if let Some(msg) = app
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.status = ChatMessageStatus::Streaming;
                        }
                    }
                    ChatEvent::Failed { request_id, error } => {
                        if let Some(msg) = app
                            .chat_messages
                            .iter_mut()
                            .find(|m| m.request_id.as_deref() == Some(&request_id))
                        {
                            msg.fail(error);
                        }
                    }
                    _ => {}
                }
            }
        }

        let asst_msg = &app.chat_messages[1];
        assert_eq!(asst_msg.status, ChatMessageStatus::Failed);
        assert!(asst_msg
            .error_detail
            .as_deref()
            .unwrap_or("")
            .contains("AI service is not connected"));
    }

    #[test]
    fn test_session_isolation_on_switch() {
        let mut app = RetinaCareApp::default();
        // Load session 0
        app.chat_messages = app.previous_sessions[0].messages.clone();
        app.active_session_id = Some(app.previous_sessions[0].id.clone());
        assert_eq!(app.active_session_id.as_deref(), Some("sess_01"));
        let sess_01_first_msg = app.chat_messages[0].text.clone();

        // Switch to session 1
        app.chat_messages = app.previous_sessions[1].messages.clone();
        app.active_session_id = Some(app.previous_sessions[1].id.clone());
        assert_eq!(app.active_session_id.as_deref(), Some("sess_02"));
        let sess_02_first_msg = app.chat_messages[0].text.clone();

        assert_ne!(sess_01_first_msg, sess_02_first_msg);
    }

    #[test]
    fn test_quad_panel_examination_initialization() {
        let app = RetinaCareApp::default();
        assert_eq!(app.exam_view.laterality, EyeLaterality::Unspecified);
        assert_eq!(app.exam_view.sync_mode, SyncMode::Synchronized);
        assert_eq!(app.exam_view.expanded_panel, None);
        assert_eq!(app.exam_view.panel_states.len(), 4);
        assert_eq!(
            app.exam_view.lesion_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            app.exam_view.vessel_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            app.exam_view.explainability_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
    }

    #[test]
    fn test_sync_mode_propagation() {
        let mut app = RetinaCareApp::default();
        app.exam_view.sync_mode = SyncMode::Synchronized;

        // Apply synchronized zoom and pan
        let new_zoom = 2.5;
        let new_pan = [45.0, -30.0];
        for state in &mut app.exam_view.panel_states {
            state.zoom = new_zoom;
            state.pan = new_pan;
        }

        for state in &app.exam_view.panel_states {
            assert_eq!(state.zoom, 2.5);
            assert_eq!(state.pan, [45.0, -30.0]);
        }
    }

    #[test]
    fn test_independent_mode_isolation() {
        let mut app = RetinaCareApp::default();
        app.exam_view.sync_mode = SyncMode::Independent;

        // Only modify Panel 0
        app.exam_view.panel_states[0].zoom = 3.0;
        app.exam_view.panel_states[0].pan = [10.0, 20.0];

        assert_eq!(app.exam_view.panel_states[0].zoom, 3.0);
        assert_eq!(app.exam_view.panel_states[1].zoom, 1.0);
        assert_eq!(app.exam_view.panel_states[2].zoom, 1.0);
        assert_eq!(app.exam_view.panel_states[3].zoom, 1.0);
    }

    #[test]
    fn test_fullscreen_expansion_and_escape() {
        let mut app = RetinaCareApp::default();
        assert_eq!(app.exam_view.expanded_panel, None);

        // Expand Panel 1 (Lesions)
        app.exam_view.expanded_panel = Some(ViewPanelId::Lesions);
        assert_eq!(app.exam_view.expanded_panel, Some(ViewPanelId::Lesions));

        // Restore to 2x2 quad grid
        app.exam_view.expanded_panel = None;
        assert_eq!(app.exam_view.expanded_panel, None);
    }

    #[test]
    fn test_laterality_switch_od_os() {
        let mut app = RetinaCareApp::default();
        assert_eq!(app.exam_view.laterality, EyeLaterality::Unspecified);

        app.exam_view.laterality = EyeLaterality::OS;
        assert_eq!(app.exam_view.laterality, EyeLaterality::OS);
    }

    #[test]
    fn test_lesion_overlay_toggles_and_opacity() {
        let mut app = RetinaCareApp::default();
        assert_eq!(app.exam_view.lesion_artifact.opacity, 0.65);
        assert!(app.exam_view.lesion_artifact.toggles.microaneurysms);

        app.exam_view.lesion_artifact.opacity = 0.90;
        app.exam_view.lesion_artifact.toggles.microaneurysms = false;

        assert_eq!(app.exam_view.lesion_artifact.opacity, 0.90);
        assert!(!app.exam_view.lesion_artifact.toggles.microaneurysms);
        assert!(app.exam_view.lesion_artifact.toggles.hemorrhages);
    }

    #[test]
    fn test_backend_readiness_mapping() {
        let app = RetinaCareApp::default();
        assert!(app.backend_readiness.is_ready());
        assert!(!app.backend_readiness.is_processing());
        assert!(app.backend_readiness.display_label().contains("Stage I"));
    }

    #[test]
    fn test_stage1_screening_submission_and_result() {
        let mut app = RetinaCareApp::default();
        app.exam_view.image_path = Some(PathBuf::from("test_fundus.png"));

        let req = crate::engine::RetinalAnalysisRequest::new(
            app.exam_view.exam_id.clone(),
            PathBuf::from("test_fundus.png"),
            Some(EyeLaterality::OD),
        );
        let req_id = req.request_id.clone();
        assert!(app.backend_service.submit(req).is_ok());

        // Process incoming event
        let event = app.backend_service.try_recv().expect("Event expected");
        match event {
            crate::engine::InferenceEvent::Stage1Completed(res) => {
                assert_eq!(res.request_id, req_id);
                assert_eq!(res.examination_id, app.exam_view.exam_id);
                assert_eq!(
                    res.decision,
                    crate::engine::Stage1GateDecision::ContinueCascade
                );
                assert!(res.screening_score >= crate::engine::STAGE1_GATE_THRESHOLD);
                assert_eq!(res.provider, "Mock CPU");
                app.stage1_result = Some(res);
            }
            other => panic!("Unexpected event: {other:?}"),
        }

        assert!(app.stage1_result.is_some());
    }

    #[test]
    fn test_stale_inference_event_rejected() {
        let mut app = RetinaCareApp::default();
        app.exam_view.exam_id = "exam_current_new".to_string();

        let stale_result = crate::engine::Stage1ScreeningResult {
            request_id: "req_old".to_string(),
            examination_id: "exam_previous_old".to_string(),
            model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "9ad51e1f".to_string(),
            provider: "CPU".to_string(),
            raw_logit: 0.5,
            screening_score: 0.622,
            threshold: crate::engine::STAGE1_GATE_THRESHOLD,
            decision: crate::engine::Stage1GateDecision::ContinueCascade,
            elapsed_ms: 120,
            preprocessing_ms: 25,
            disclaimer: "Disclaimer".to_string(),
        };

        // When a result arrives with an examination ID mismatch, it is rejected
        if stale_result.examination_id == app.exam_view.exam_id {
            app.stage1_result = Some(stale_result);
        }

        assert!(app.stage1_result.is_none());
    }

    #[test]
    fn test_examination_isolation_on_new_consultation() {
        let mut app = RetinaCareApp::default();
        app.stage1_result = Some(crate::engine::Stage1ScreeningResult {
            request_id: "req_1".to_string(),
            examination_id: app.exam_view.exam_id.clone(),
            model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "9ad51e1f".to_string(),
            provider: "CPU".to_string(),
            raw_logit: 1.0,
            screening_score: 0.73,
            threshold: crate::engine::STAGE1_GATE_THRESHOLD,
            decision: crate::engine::Stage1GateDecision::ContinueCascade,
            elapsed_ms: 100,
            preprocessing_ms: 20,
            disclaimer: "Disclaimer".to_string(),
        });

        // New consultation clears results
        app.stage1_result = None;
        app.stage1_error = None;
        app.active_screening_request = None;

        assert!(app.stage1_result.is_none());
        assert!(app.stage1_error.is_none());
    }

    #[test]
    fn test_cancellation_state_transition() {
        let app = RetinaCareApp::default();
        app.backend_service.cancel("req-to-cancel");
        let event = app.backend_service.try_recv().expect("Cancelled event");
        assert_eq!(
            event,
            crate::engine::InferenceEvent::Cancelled {
                request_id: "req-to-cancel".to_string()
            }
        );
    }

    #[test]
    fn test_unintegrated_stage2_stage3_remain_unavailable() {
        let app = RetinaCareApp::default();
        assert_eq!(
            app.exam_view.lesion_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            app.exam_view.vessel_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            app.exam_view.explainability_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
    }

    #[test]
    fn test_report_view_model_generation_from_stage1() {
        let stage1 = crate::engine::Stage1ScreeningResult {
            request_id: "req_vm_01".to_string(),
            examination_id: "exam_vm_01".to_string(),
            model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "9ad51e1fa4c8e7631da7e0524cbbf5b8b64bf2ab77f7226db2c214ee6b680c65"
                .to_string(),
            provider: "CPU Execution Provider".to_string(),
            raw_logit: 1.45,
            screening_score: 0.8099,
            threshold: crate::engine::STAGE1_GATE_THRESHOLD,
            decision: crate::engine::Stage1GateDecision::ContinueCascade,
            elapsed_ms: 180,
            preprocessing_ms: 25,
            disclaimer: crate::engine::MANDATORY_RESEARCH_CAVEAT.to_string(),
        };

        let vm = crate::engine::ReportViewModel::new(
            "exam_vm_01",
            "PATIENT_99",
            EyeLaterality::OD,
            None,
            Some(stage1),
        );

        assert!(vm.has_stage1_findings());
        assert_eq!(
            vm.gate_decision(),
            Some(crate::engine::Stage1GateDecision::ContinueCascade)
        );
        assert_eq!(vm.screening_score(), Some(0.8099));
        assert!(vm.validate_isolation("exam_vm_01", "PATIENT_99"));
        assert!(!vm.validate_isolation("exam_vm_02", "PATIENT_99"));
        assert!(vm
            .non_diagnostic_caveat
            .contains("For investigational screening only"));
    }

    #[test]
    fn test_session_export_scope_isolation() {
        let tmp = std::env::temp_dir().join("retinacare_test_session_scope");
        let exam = RetinalExaminationView {
            exam_id: "exam_scope_01".to_string(),
            patient_code: "PATIENT_SCOPE_01".to_string(),
            ..Default::default()
        };

        let stage1 = crate::engine::Stage1ScreeningResult {
            request_id: "req_scope_01".to_string(),
            examination_id: "exam_scope_01".to_string(),
            model_id: "hierarchiretina-stage1-convnextv2l-768".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "hash123".to_string(),
            provider: "CPU".to_string(),
            raw_logit: -1.2,
            screening_score: 0.12,
            threshold: crate::engine::STAGE1_GATE_THRESHOLD,
            decision: crate::engine::Stage1GateDecision::NegativeRoute,
            elapsed_ms: 95,
            preprocessing_ms: 15,
            disclaimer: crate::engine::MANDATORY_RESEARCH_CAVEAT.to_string(),
        };

        let export_res = crate::engine::export_scoped_patient_session(&tmp, &exam, Some(&stage1));
        assert!(export_res.is_ok());
        let session_dir = export_res.unwrap();
        assert!(session_dir.exists());

        // Verify plaintext unencrypted export files
        let manifest_content = std::fs::read_to_string(session_dir.join("manifest.json")).unwrap();
        assert!(manifest_content.contains("PATIENT_SCOPE_01"));
        assert!(manifest_content.contains("exam_scope_01"));
        assert!(manifest_content.contains("PLAINTEXT_SENSITIVE_UNENCRYPTED_JSON"));

        let findings_content =
            std::fs::read_to_string(session_dir.join("screening_findings.json")).unwrap();
        assert!(findings_content.contains("NegativeRoute"));
        assert!(findings_content.contains("Unavailable"));

        let _ = std::fs::remove_dir_all(session_dir);
    }

    #[test]
    fn test_pdf_export_state_machine_with_mock() {
        use crate::engine::PdfExportService;
        let service = crate::engine::MockPdfExportService::new_success();
        let target_pdf = std::env::temp_dir().join("test_report_state_machine.pdf");
        let req = crate::engine::ReportExportRequest {
            request_id: "req_sm_01".to_string(),
            examination_id: "exam_sm_01".to_string(),
            patient_code: "PAT_SM_01".to_string(),
            destination_path: target_pdf.clone(),
            include_original_image: true,
            include_findings_json: true,
            include_provenance_meta: true,
            document: None,
        };

        let rx = service.submit_export(req).expect("Submit export");
        let mut got_completed = false;
        while let Ok(ev) = rx.recv() {
            if let crate::engine::ExportEvent::Completed { path, bytes, .. } = ev {
                assert_eq!(path, target_pdf);
                assert!(bytes > 0);
                got_completed = true;
            }
        }
        assert!(got_completed);
        assert!(target_pdf.exists());
        let _ = std::fs::remove_file(target_pdf);
    }

    #[test]
    fn test_live_services_readiness_reporting() {
        use crate::engine::{AuditLedgerService, PdfExportService};
        let pdf_svc = crate::engine::LiveCodexPdfService::default();
        assert!(matches!(
            pdf_svc.readiness(),
            crate::engine::PdfServiceReadiness::Ready { .. }
        ));

        // A locked database makes the audit ledger truthfully unavailable, never empty-but-ready.
        let store = crate::engine::store::SharedStore::new(
            std::env::temp_dir().join("rc-main-audit-locked/db.sqlite"),
        );
        let audit_svc = crate::engine::LiveCodexAuditService::with_store(store);
        match audit_svc.readiness() {
            crate::engine::AuditLedgerReadiness::Unavailable { reason } => {
                assert!(reason.contains("locked"));
            }
            _ => panic!("a locked database must report the audit ledger unavailable"),
        }
    }

    #[test]
    fn test_patient_page_navigation() {
        let mut app = RetinaCareApp::default();
        assert_eq!(app.active_page, AppPage::Home);

        app.active_page = AppPage::Patients;
        assert_eq!(app.active_page, AppPage::Patients);
    }

    #[test]
    fn test_patient_selection_and_isolation_on_switch() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        // Simulate loaded image and result for Patient A
        app.active_patient_id = Some(1);
        app.exam_view.patient_code = "PATIENT_A".to_string();
        app.uploaded_filename = Some("patient_a_scan.png".to_string());
        app.active_request_id = Some("req_a".to_string());
        app.stage1_result = Some(crate::engine::Stage1ScreeningResult {
            request_id: "req_a".to_string(),
            examination_id: "exam_a".to_string(),
            model_id: "model".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "hash".to_string(),
            provider: "CPU".to_string(),
            raw_logit: -1.0,
            screening_score: 0.1,
            threshold: 0.2391716,
            decision: crate::engine::Stage1GateDecision::NegativeRoute,
            elapsed_ms: 50,
            preprocessing_ms: 10,
            disclaimer: "Disclaimer".to_string(),
        });

        // Switch to Patient B (ID: 2)
        app.handle_patients_action(ui::PatientsAction::SelectPatient(Some(2)), &ctx);

        // Verify isolation
        assert_eq!(app.active_patient_id, Some(2));
        assert_eq!(app.exam_view.patient_code, "TEST-P002");
        assert!(app.uploaded_filename.is_none());
        assert!(app.active_request_id.is_none());
        assert!(app.stage1_result.is_none());
    }

    #[test]
    fn test_patient_save_and_dirty_flag_reset() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        app.handle_patients_action(ui::PatientsAction::SelectPatient(Some(1)), &ctx);
        assert_eq!(app.patients_view_state.edit_code, "TEST-P001");

        // Save update
        app.handle_patients_action(
            ui::PatientsAction::SavePatient {
                id: 1,
                code: "TEST-P001-MOD".to_string(),
                name: "Updated Name".to_string(),
            },
            &ctx,
        );

        assert!(!app.patients_view_state.edit_dirty);
        assert!(app.patients_view_state.edit_status_msg.is_some());
        let (msg, is_err) = app.patients_view_state.edit_status_msg.as_ref().unwrap();
        assert!(!is_err);
        assert!(msg.contains("Saved patient"));

        let updated = app.patient_service.get_patient(1).unwrap();
        assert_eq!(updated.patient_code, "TEST-P001-MOD");
        assert_eq!(updated.display_name, "Updated Name");
    }

    #[test]
    fn test_create_examination_associated_with_patient() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        let new_exam = crate::engine::ExaminationSummary {
            id: 0,
            patient_id: 2,
            patient_code: "TEST-P002".to_string(),
            examined_at: "2026-10-28T04:00:00Z".to_string(),
            operator: "Dr. Ophthalmologist".to_string(),
            notes: "Routine follow-up evaluation.".to_string(),
            eye: "OS".to_string(),
            image_path: "".to_string(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: true,
        };

        app.handle_patients_action(ui::PatientsAction::CreateExamination(new_exam), &ctx);

        let exams = app.patient_service.list_examinations(2).unwrap();
        assert_eq!(exams.len(), 2);
        assert_eq!(exams[1].eye, "OS");
        assert_eq!(exams[1].operator, "Dr. Ophthalmologist");
    }

    #[test]
    fn test_open_exam_in_report_and_viewer_navigation() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        app.active_page = AppPage::Patients;

        // Open Exam 1 in Report mode
        app.handle_patients_action(
            ui::PatientsAction::OpenInReport {
                patient_id: 1,
                exam_id: 1,
            },
            &ctx,
        );

        assert_eq!(app.active_page, AppPage::Report);
        assert!(app.exam_view.report_view_mode);
        assert_eq!(app.active_patient_id, Some(1));
        assert_eq!(app.active_examination_id, Some(1));

        // Return to Patients
        app.active_page = AppPage::Patients;

        // Open Exam 1 in Quad Viewer mode
        app.handle_patients_action(
            ui::PatientsAction::OpenInViewer {
                patient_id: 1,
                exam_id: 1,
            },
            &ctx,
        );

        assert_eq!(app.active_page, AppPage::Report);
        assert!(!app.exam_view.report_view_mode);
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::QuadViewer
        );
    }

    #[test]
    fn test_navigation_subroute_stack_directory_to_profile_to_exam_detail() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        assert_eq!(
            app.patients_view_state.current_subroute,
            crate::domain::PatientSubroute::Directory
        );

        // Navigate to Patient Profile
        app.handle_patients_action(
            ui::PatientsAction::NavigateSubroute(crate::domain::PatientSubroute::Profile {
                patient_id: 1,
            }),
            &ctx,
        );
        assert_eq!(
            app.patients_view_state.current_subroute,
            crate::domain::PatientSubroute::Profile { patient_id: 1 }
        );
        assert_eq!(app.active_patient_id, Some(1));
        assert_eq!(app.exam_view.patient_code, "TEST-P001");

        // Navigate to Examination Detail
        app.handle_patients_action(
            ui::PatientsAction::NavigateSubroute(crate::domain::PatientSubroute::ExamDetail {
                patient_id: 1,
                examination_id: 1,
            }),
            &ctx,
        );
        assert_eq!(
            app.patients_view_state.current_subroute,
            crate::domain::PatientSubroute::ExamDetail {
                patient_id: 1,
                examination_id: 1,
            }
        );
        assert_eq!(app.active_examination_id, Some(1));

        // Navigate Back to Profile
        app.handle_patients_action(
            ui::PatientsAction::NavigateSubroute(crate::domain::PatientSubroute::Profile {
                patient_id: 1,
            }),
            &ctx,
        );
        assert_eq!(
            app.patients_view_state.current_subroute,
            crate::domain::PatientSubroute::Profile { patient_id: 1 }
        );

        // Navigate Back to Directory
        app.handle_patients_action(
            ui::PatientsAction::NavigateSubroute(crate::domain::PatientSubroute::Directory),
            &ctx,
        );
        assert_eq!(
            app.patients_view_state.current_subroute,
            crate::domain::PatientSubroute::Directory
        );
    }

    #[test]
    fn test_navigation_report_subroutes_pdf_session_audit_back() {
        let mut app = RetinaCareApp::default();
        let _ctx = egui::Context::default();

        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::QuadViewer
        );

        // Navigate to PDF Export
        app.exam_view.current_subroute = crate::domain::ReportSubroute::PdfExport;
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::PdfExport
        );

        // Back to Quad Report Workspace
        app.exam_view.report_view_mode = false;
        app.exam_view.current_subroute = if app.exam_view.report_view_mode {
            crate::domain::ReportSubroute::ReportOverview
        } else {
            crate::domain::ReportSubroute::QuadViewer
        };
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::QuadViewer
        );

        // Switch to Overview and navigate to Session Export
        app.exam_view.report_view_mode = true;
        app.exam_view.current_subroute = crate::domain::ReportSubroute::SessionExport;
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::SessionExport
        );

        // Back to Overview Report Workspace
        app.exam_view.current_subroute = if app.exam_view.report_view_mode {
            crate::domain::ReportSubroute::ReportOverview
        } else {
            crate::domain::ReportSubroute::QuadViewer
        };
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::ReportOverview
        );

        // Navigate to Audit History
        app.exam_view.current_subroute = crate::domain::ReportSubroute::AuditHistory;
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::AuditHistory
        );
    }

    #[test]
    fn test_preview_patient_selection() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        assert!(app.patients_view_state.preview_patient_id.is_none());

        app.handle_patients_action(ui::PatientsAction::PreviewPatient(Some(2)), &ctx);
        assert_eq!(app.patients_view_state.preview_patient_id, Some(2));

        app.handle_patients_action(ui::PatientsAction::PreviewPatient(None), &ctx);
        assert!(app.patients_view_state.preview_patient_id.is_none());
    }

    #[test]
    fn test_idle_heartbeat_and_renderer_configuration() {
        // Test default renderer selection: Glow by default, Wgpu when requested
        let default_renderer = if let Ok(r) = std::env::var("RETINACARE_RENDERER") {
            if r.to_lowercase() == "wgpu" {
                eframe::Renderer::Wgpu
            } else {
                eframe::Renderer::Glow
            }
        } else {
            eframe::Renderer::Glow
        };
        assert_eq!(default_renderer, eframe::Renderer::Glow);

        // Verify egui context registers request_repaint_after without panic
        let ctx = egui::Context::default();
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        assert!(ctx.has_requested_repaint());
    }

    #[test]
    fn test_timeline_page_navigation() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();
        assert_eq!(app.active_page, AppPage::Home);

        // Switch to Timeline
        app.active_page = AppPage::Timeline;
        assert_eq!(app.active_page, AppPage::Timeline);
        assert_eq!(
            app.timeline_view_state.current_subroute,
            crate::domain::TimelineSubroute::Overview
        );

        // Test timeline subroute navigation
        app.handle_timeline_action(
            crate::domain::TimelineAction::NavigateSubroute(
                crate::domain::TimelineSubroute::EventDetail {
                    event_id: "ev_cap_1".to_string(),
                },
            ),
            &ctx,
        );
        assert_eq!(
            app.timeline_view_state.current_subroute,
            crate::domain::TimelineSubroute::EventDetail {
                event_id: "ev_cap_1".to_string(),
            }
        );
    }

    #[test]
    fn test_timeline_cross_patient_isolation() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        // Select Patient 1 and an event
        app.handle_timeline_action(crate::domain::TimelineAction::SelectPatient(Some(1)), &ctx);
        assert_eq!(app.active_patient_id, Some(1));
        app.handle_timeline_action(
            crate::domain::TimelineAction::SelectEvent(Some("ev_cap_1".to_string())),
            &ctx,
        );
        assert_eq!(
            app.timeline_view_state.selected_event_id.as_deref(),
            Some("ev_cap_1")
        );

        // Switch to Patient 2: must purge selected event to prevent cross-patient bleed
        app.handle_timeline_action(crate::domain::TimelineAction::SelectPatient(Some(2)), &ctx);
        assert_eq!(app.active_patient_id, Some(2));
        assert!(app.timeline_view_state.selected_event_id.is_none());
    }

    #[test]
    fn test_timeline_time_ruler_zoom_and_fit() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();
        assert_eq!(app.timeline_view_state.zoom_level, 1.0);

        // Zoom In
        app.handle_timeline_action(crate::domain::TimelineAction::ZoomIn, &ctx);
        assert!(app.timeline_view_state.zoom_level > 1.0);

        // Reset Zoom
        app.handle_timeline_action(crate::domain::TimelineAction::ResetZoom, &ctx);
        assert_eq!(app.timeline_view_state.zoom_level, 1.0);

        // Zoom Out
        app.handle_timeline_action(crate::domain::TimelineAction::ZoomOut, &ctx);
        assert!(app.timeline_view_state.zoom_level < 1.0);
    }

    #[test]
    fn test_history_subroute_navigation_and_back() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();
        assert_eq!(app.active_page, AppPage::Home);

        // Navigate to History page
        app.active_page = AppPage::PreviousChats;
        assert_eq!(app.active_page, AppPage::PreviousChats);
        assert_eq!(
            app.history_view_state.current_subroute,
            crate::domain::HistorySubroute::List
        );

        // Open conversation detail
        app.handle_history_action(
            crate::domain::HistoryAction::NavigateSubroute(
                crate::domain::HistorySubroute::Detail {
                    conversation_id: "sess_01".to_string(),
                },
            ),
            &ctx,
        );
        assert_eq!(
            app.history_view_state.current_subroute,
            crate::domain::HistorySubroute::Detail {
                conversation_id: "sess_01".to_string(),
            }
        );

        // Navigate back to list
        app.handle_history_action(
            crate::domain::HistoryAction::NavigateSubroute(crate::domain::HistorySubroute::List),
            &ctx,
        );
        assert_eq!(
            app.history_view_state.current_subroute,
            crate::domain::HistorySubroute::List
        );
    }

    #[test]
    fn test_history_read_only_replay_no_rerun() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        let initial_stage = app.analysis_stage;
        let initial_active_req = app.active_request_id.clone();
        let initial_screening_req = app.active_screening_request.clone();

        // Navigating into History conversation detail must NEVER trigger inference or model execution
        app.handle_history_action(
            crate::domain::HistoryAction::NavigateSubroute(
                crate::domain::HistorySubroute::Detail {
                    conversation_id: "sess_02".to_string(),
                },
            ),
            &ctx,
        );

        assert_eq!(app.analysis_stage, initial_stage);
        assert_eq!(app.active_request_id, initial_active_req);
        assert_eq!(app.active_screening_request, initial_screening_req);
    }

    #[test]
    fn test_timeline_deep_links_to_report_and_quad() {
        let mut app = RetinaCareApp::default();
        let ctx = egui::Context::default();

        // Deep link into Quad viewer for exam 101
        app.handle_timeline_action(
            crate::domain::TimelineAction::OpenInQuad {
                examination_id: 101,
            },
            &ctx,
        );
        assert_eq!(app.active_page, AppPage::Report);
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::QuadViewer
        );
        assert_eq!(app.active_examination_id, Some(101));

        // Deep link into Clinical Report overview for patient 2, exam 102
        app.handle_timeline_action(
            crate::domain::TimelineAction::OpenInReport {
                patient_id: 2,
                examination_id: 102,
            },
            &ctx,
        );
        assert_eq!(app.active_page, AppPage::Report);
        assert_eq!(
            app.exam_view.current_subroute,
            crate::domain::ReportSubroute::ReportOverview
        );
        assert_eq!(app.active_patient_id, Some(2));
        assert_eq!(app.active_examination_id, Some(102));
    }
}

/// Chat send held back until the user says which eye the attached image(s) show.
struct EyeChoice {
    prompt: String,
    files: Vec<LocalAttachment>,
    /// Patient open when the images were attached; a change cancels the choice.
    patient: Option<i64>,
    thumbs: Vec<Option<egui::TextureHandle>>,
    first_is_os: bool,
    /// When the chat send created this choice: the dialog waits for the attachment menu's fade.
    since: std::time::Instant,
}

fn eye_thumb(ctx: &egui::Context, path: &std::path::Path) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.thumbnail(150, 150).to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ctx.load_texture(
        format!("eye_thumb_{}", path.display()),
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        Default::default(),
    ))
}
