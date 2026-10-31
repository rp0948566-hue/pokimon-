//! Professional Retinal Image Viewer and Quad-Panel Report Workspace for RetinaCare.
use super::theme;
use crate::domain::{
    ArtifactLoadStatus, EyeLaterality, ReportSubroute, RetinalExaminationView, SyncMode,
    ViewPanelId,
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Key, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2,
};

/// Preserves image aspect ratio inside any bounding box without distortion
pub fn aspect_fit(img_w: f32, img_h: f32, container: Rect) -> Rect {
    let aspect = img_w / img_h.max(1.0);
    let container_aspect = container.width() / container.height().max(1.0);
    if aspect > container_aspect {
        let h = container.width() / aspect;
        Rect::from_center_size(container.center(), Vec2::new(container.width(), h))
    } else {
        let w = container.height() * aspect;
        Rect::from_center_size(container.center(), Vec2::new(w, container.height()))
    }
}

/// Computes the base fitted rect and the final display rect with zoom and pan applied
pub fn compute_display_rect(
    viewport: Rect,
    img_w: f32,
    img_h: f32,
    zoom: f32,
    pan: Vec2,
) -> (Rect, Rect) {
    let fit_base = aspect_fit(img_w, img_h, viewport.shrink(12.0));
    let scaled_size = fit_base.size() * zoom;
    // The image may never leave the viewport: at zoom 1 there is no pan at all.
    let slack = ((scaled_size - viewport.size()) / 2.0).max(Vec2::ZERO);
    let pan = Vec2::new(
        pan.x.clamp(-slack.x, slack.x),
        pan.y.clamp(-slack.y, slack.y),
    );
    let target_center = fit_base.center() + pan;
    let display_rect = Rect::from_center_size(target_center, scaled_size);
    (fit_base, display_rect)
}

/// Converts a screen cursor position into discrete image pixel coordinates and normalized (0..1) coordinates.
/// Strictly returns `None` if the cursor is outside the displayed image content (e.g. over letterbox margins).
pub fn screen_to_image_coords(
    pos: Pos2,
    display_rect: Rect,
    img_w: f32,
    img_h: f32,
) -> Option<((u32, u32), (f32, f32))> {
    if !display_rect.contains(pos) || display_rect.width() <= 0.0 || display_rect.height() <= 0.0 {
        return None;
    }
    let norm_u = ((pos.x - display_rect.min.x) / display_rect.width()).clamp(0.0, 1.0);
    let norm_v = ((pos.y - display_rect.min.y) / display_rect.height()).clamp(0.0, 1.0);

    let max_x = (img_w as u32).saturating_sub(1);
    let max_y = (img_h as u32).saturating_sub(1);

    let px = ((norm_u * img_w).floor() as u32).min(max_x);
    let py = ((norm_v * img_h).floor() as u32).min(max_y);

    Some(((px, py), (norm_u, norm_v)))
}

/// Calculates cursor-centered zoom: keeps the image coordinate under the cursor stationary while scaling.
pub fn cursor_centered_zoom(
    mouse_pos: Pos2,
    zoom_mult: f32,
    current_zoom: f32,
    current_pan: Vec2,
    fit_base: Rect,
    min_zoom: f32,
    max_zoom: f32,
) -> (f32, Vec2) {
    let new_zoom = (current_zoom * zoom_mult).clamp(min_zoom, max_zoom);
    if (new_zoom - current_zoom).abs() < f32::EPSILON {
        return (current_zoom, current_pan);
    }

    let old_center = fit_base.center() + current_pan;
    let offset_from_center = mouse_pos - old_center;
    let zoom_ratio = new_zoom / current_zoom.max(0.001);

    let new_center = mouse_pos - offset_from_center * zoom_ratio;
    let new_pan = new_center - fit_base.center();

    (new_zoom, new_pan)
}

/// User interaction outcome from the quad-panel workspace
#[derive(Debug, Clone, PartialEq)]
pub enum ViewerAction {
    None,
    UploadImageRequested,
    LateralityChanged(EyeLaterality),
    SyncModeChanged(SyncMode),
    ExpandPanel(Option<ViewPanelId>),
    ResetAll,
    FitAll,
    SyncZoomAndPan { zoom: f32, pan: [f32; 2] },
    RunStage1Inference,
    CancelInference,
    DismissResult,
    ToggleReportViewMode,
    OpenExportPdfDialog,
    OpenSessionExportDialog,
    ToggleAuditLedger,
    SubmitPdfExport,
    SubmitSessionExport,
    CloseModals,
    NavigateReportSubroute(ReportSubroute),
    BackToReportWorkspace,
}

/// Renders the complete Retinal Visualization & Clinical Report Workspace
#[allow(clippy::too_many_arguments)]
pub fn render_retinal_workspace(
    ui: &mut Ui,
    ctx: &egui::Context,
    main_rect: Rect,
    exam: &mut RetinalExaminationView,
    mode_textures: &[Option<TextureHandle>; 4],
    uploaded_filename: Option<&str>,
    uploaded_dimensions: Option<[usize; 2]>,
    backend_readiness: &crate::engine::BackendReadiness,
    stage1_result: Option<&crate::engine::Stage1ScreeningResult>,
    stage1_error: Option<&str>,
    pdf_readiness: &crate::engine::PdfServiceReadiness,
    audit_readiness: &crate::engine::AuditLedgerReadiness,
    export_state: &crate::engine::ExportState,
    audit_events: &[crate::engine::AuditEvent],
    session_export_status: Option<&str>,
) -> ViewerAction {
    let mut action = ViewerAction::None;

    // Check keyboard shortcuts
    if exam.expanded_panel.is_some() && ui.input(|i| i.key_pressed(Key::Escape)) {
        action = ViewerAction::ExpandPanel(None);
    }
    if (exam.export_dialog_open || exam.audit_ledger_open)
        && ui.input(|i| i.key_pressed(Key::Escape))
    {
        exam.export_dialog_open = false;
        exam.audit_ledger_open = false;
        action = ViewerAction::CloseModals;
    }

    // Top Action Toolbar
    let toolbar_height = 42.0;
    let toolbar_rect =
        Rect::from_min_size(main_rect.min, Vec2::new(main_rect.width(), toolbar_height));
    let has_image = mode_textures[0].is_some() || exam.image_path.is_some();

    match exam.current_subroute {
        ReportSubroute::PdfExport => {
            render_report_subroute_toolbar(ui, toolbar_rect, exam, "📄 PDF Export", &mut action);
            let workspace_top = main_rect.min.y + toolbar_height + 8.0;
            let workspace_rect =
                Rect::from_min_max(Pos2::new(main_rect.min.x, workspace_top), main_rect.max);
            render_pdf_export_screen(
                ui,
                workspace_rect,
                exam,
                pdf_readiness,
                export_state,
                &mut action,
            );
        }
        ReportSubroute::SessionExport => {
            render_report_subroute_toolbar(
                ui,
                toolbar_rect,
                exam,
                "📦 Session Dossier Export",
                &mut action,
            );
            let workspace_top = main_rect.min.y + toolbar_height + 8.0;
            let workspace_rect =
                Rect::from_min_max(Pos2::new(main_rect.min.x, workspace_top), main_rect.max);
            render_session_export_screen(
                ui,
                workspace_rect,
                exam,
                session_export_status,
                &mut action,
            );
        }
        ReportSubroute::AuditHistory => {
            render_report_subroute_toolbar(
                ui,
                toolbar_rect,
                exam,
                "📋 Append-Only Audit Ledger",
                &mut action,
            );
            let workspace_top = main_rect.min.y + toolbar_height + 8.0;
            let workspace_rect =
                Rect::from_min_max(Pos2::new(main_rect.min.x, workspace_top), main_rect.max);
            render_audit_history_screen(
                ui,
                workspace_rect,
                exam,
                audit_readiness,
                audit_events,
                &mut action,
            );
        }
        ReportSubroute::QuadViewer | ReportSubroute::ReportOverview => {
            let is_report =
                exam.current_subroute == ReportSubroute::ReportOverview || exam.report_view_mode;
            render_workspace_toolbar(
                ui,
                toolbar_rect,
                exam,
                uploaded_filename,
                uploaded_dimensions,
                backend_readiness,
                has_image,
                &mut action,
            );

            let mut banner_h = 0.0;
            let cascade_banner = exam.cascade.available
                || exam.cascade.is_active()
                || exam.cascade.outcome.is_some()
                || exam.cascade.error.is_some();
            if !is_report && cascade_banner {
                let lines = super::cascade_cards::compact_lines(&exam.cascade, has_image);
                let h = super::cascade_cards::measure(ui, &lines, main_rect.width());
                let card_rect = Rect::from_min_size(
                    Pos2::new(main_rect.min.x, main_rect.min.y + toolbar_height + 6.0),
                    Vec2::new(main_rect.width(), h),
                );
                super::cascade_cards::paint(ui, card_rect, &lines);
                banner_h = h + 4.0;
            } else if !is_report {
                if let Some(res) = stage1_result {
                    banner_h = 68.0;
                    let card_rect = Rect::from_min_size(
                        Pos2::new(main_rect.min.x, main_rect.min.y + toolbar_height + 6.0),
                        Vec2::new(main_rect.width(), 64.0),
                    );
                    render_stage1_result_card(ui, card_rect, res, &mut action);
                } else if let Some(err) = stage1_error {
                    banner_h = 42.0;
                    let err_rect = Rect::from_min_size(
                        Pos2::new(main_rect.min.x, main_rect.min.y + toolbar_height + 6.0),
                        Vec2::new(main_rect.width(), 36.0),
                    );
                    render_stage1_error_card(ui, err_rect, err, &mut action);
                }
            }

            let workspace_top = main_rect.min.y + toolbar_height + banner_h + 8.0;
            let workspace_rect =
                Rect::from_min_max(Pos2::new(main_rect.min.x, workspace_top), main_rect.max);

            if is_report {
                render_clinical_report_overview(
                    ui,
                    workspace_rect,
                    exam,
                    mode_textures,
                    uploaded_filename,
                    uploaded_dimensions,
                    stage1_result,
                    stage1_error,
                    session_export_status,
                    &mut action,
                );
            } else if let Some(expanded_id) = exam.expanded_panel {
                render_single_panel(
                    ui,
                    ctx,
                    workspace_rect,
                    expanded_id,
                    exam,
                    mode_textures,
                    uploaded_dimensions,
                    true,
                    &mut action,
                );
            } else {
                // Otherwise render 2x2 Quad-Panel Grid
                let is_narrow = workspace_rect.width() < 720.0;
                if is_narrow {
                    // Confined to the workspace rect: a bare `show(ui, ..)` starts at the page
                    // origin and paints the panels over the toolbar and banner.
                    let mut narrow_ui =
                        ui.new_child(egui::UiBuilder::new().max_rect(workspace_rect));
                    egui::ScrollArea::vertical()
                        .id_salt("quad_panel_scroll")
                        .auto_shrink([false, false])
                        .show(&mut narrow_ui, |ui| {
                            let panel_height = 360.0;
                            for idx in 0..4 {
                                let panel_id = match idx {
                                    0 => ViewPanelId::Original,
                                    1 => ViewPanelId::Lesions,
                                    2 => ViewPanelId::Vessels,
                                    _ => ViewPanelId::Explainability,
                                };
                                let panel_rect = ui
                                    .allocate_space(Vec2::new(ui.available_width(), panel_height))
                                    .1;
                                render_single_panel(
                                    ui,
                                    ctx,
                                    panel_rect,
                                    panel_id,
                                    exam,
                                    mode_textures,
                                    uploaded_dimensions,
                                    false,
                                    &mut action,
                                );
                                ui.add_space(8.0);
                            }
                        });
                } else {
                    // Standard balanced 2x2 grid
                    let gap = 10.0;
                    let col_w = ((workspace_rect.width() - gap) / 2.0).max(100.0);
                    let row_h = ((workspace_rect.height() - gap) / 2.0).max(100.0);

                    let panels = [
                        (ViewPanelId::Original, 0, 0),
                        (ViewPanelId::Lesions, 1, 0),
                        (ViewPanelId::Vessels, 0, 1),
                        (ViewPanelId::Explainability, 1, 1),
                    ];

                    for (panel_id, col, row) in panels {
                        let px = workspace_rect.min.x + (col as f32) * (col_w + gap);
                        let py = workspace_rect.min.y + (row as f32) * (row_h + gap);
                        let panel_rect =
                            Rect::from_min_size(Pos2::new(px, py), Vec2::new(col_w, row_h));

                        render_single_panel(
                            ui,
                            ctx,
                            panel_rect,
                            panel_id,
                            exam,
                            mode_textures,
                            uploaded_dimensions,
                            false,
                            &mut action,
                        );
                    }
                }
            }
        }
    }

    // Modal Dialog Overlays
    if exam.export_dialog_open {
        render_pdf_export_modal(
            ui,
            main_rect,
            exam,
            pdf_readiness,
            export_state,
            &mut action,
        );
    }
    if exam.audit_ledger_open {
        render_audit_ledger_modal(
            ui,
            main_rect,
            exam,
            audit_readiness,
            audit_events,
            &mut action,
        );
    }

    action
}

/// Renders the top workspace toolbar with eye laterality, view mode switcher, export buttons, sync mode, metadata, upload button, and Stage 1 screening action
#[allow(clippy::too_many_arguments)]
/// Left cluster (views, eye, sync, report routes) scrolls horizontally when it does not fit; the
/// Run/Cancel action stays pinned on the right so it is never clipped or covered.
fn render_workspace_toolbar(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    uploaded_filename: Option<&str>,
    uploaded_dimensions: Option<[usize; 2]>,
    backend_readiness: &crate::engine::BackendReadiness,
    has_image: bool,
    action: &mut ViewerAction,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        theme::TOP_BAR_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    const LEFT_NEEDED: f32 = 850.0;
    const LEFT_MIN: f32 = 260.0;
    const RUN_W: f32 = 195.0;
    const UPLOAD_W: f32 = 154.0;
    const PILL_W: f32 = 153.0;
    const META_W: f32 = 153.0;
    let avail = rect.width();
    let mut flags = ToolbarFlags {
        upload: avail >= 640.0,
        pill: false,
        meta: false,
    };
    let mut right_w = 10.0 + RUN_W + if flags.upload { UPLOAD_W } else { 0.0 };
    if avail - (right_w + PILL_W) >= LEFT_MIN {
        flags.pill = true;
        right_w += PILL_W;
    }
    if avail - (right_w + META_W) >= LEFT_NEEDED {
        flags.meta = true;
        right_w += META_W;
    }
    let right_rect = Rect::from_min_max(Pos2::new(rect.max.x - right_w, rect.min.y), rect.max);
    render_workspace_toolbar_part(
        ui,
        right_rect,
        exam,
        uploaded_filename,
        uploaded_dimensions,
        backend_readiness,
        has_image,
        ToolbarPart::Right,
        flags,
        action,
    );
    let left_view = Rect::from_min_max(rect.min, Pos2::new(right_rect.min.x - 6.0, rect.max.y));
    let content_w = LEFT_NEEDED.max(left_view.width());
    let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left_view));
    egui::ScrollArea::horizontal()
        .id_salt("workspace_toolbar_left_scroll")
        .auto_shrink([false, false])
        .show(&mut left_ui, |ui| {
            let inner = ui.allocate_space(Vec2::new(content_w, rect.height())).1;
            render_workspace_toolbar_part(
                ui,
                inner,
                exam,
                uploaded_filename,
                uploaded_dimensions,
                backend_readiness,
                has_image,
                ToolbarPart::Left,
                flags,
                action,
            );
        });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolbarPart {
    Left,
    Right,
}

/// Which optional right-hand items fit. The Run/Cancel button is never dropped.
#[derive(Clone, Copy)]
struct ToolbarFlags {
    upload: bool,
    pill: bool,
    meta: bool,
}

#[allow(clippy::too_many_arguments)]
fn render_workspace_toolbar_part(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    uploaded_filename: Option<&str>,
    uploaded_dimensions: Option<[usize; 2]>,
    backend_readiness: &crate::engine::BackendReadiness,
    has_image: bool,
    part: ToolbarPart,
    flags: ToolbarFlags,
    action: &mut ViewerAction,
) {
    let mid_y = rect.center().y;
    if part == ToolbarPart::Left {
        let mut cur_x = rect.min.x + 10.0;

        // View Mode Switcher: [⊞ Quad Grid] / [📋 Report]
        let is_report =
            exam.report_view_mode || exam.current_subroute == ReportSubroute::ReportOverview;
        let quad_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(76.0, 26.0));
        let rep_rect =
            Rect::from_min_size(Pos2::new(cur_x + 80.0, mid_y - 13.0), Vec2::new(76.0, 26.0));
        cur_x += 162.0;

        let quad_resp = ui.interact(quad_rect, Id::new("view_mode_quad_btn"), Sense::click());
        let rep_resp = ui.interact(rep_rect, Id::new("view_mode_report_btn"), Sense::click());

        ui.painter().rect(
            quad_rect,
            CornerRadius::same(6),
            if !is_report {
                theme::CARD_HOVER()
            } else {
                theme::INNER_TILE()
            },
            Stroke::new(
                1.0,
                if !is_report {
                    theme::ACCENT_BLUE()
                } else {
                    theme::BORDER()
                },
            ),
            StrokeKind::Inside,
        );
        ui.painter().text(
            quad_rect.center(),
            Align2::CENTER_CENTER,
            "⊞ Quad",
            FontId::proportional(11.0),
            if !is_report {
                theme::TEXT_PRIMARY()
            } else {
                theme::TEXT_MUTED()
            },
        );

        ui.painter().rect(
            rep_rect,
            CornerRadius::same(6),
            if is_report {
                theme::CARD_HOVER()
            } else {
                theme::INNER_TILE()
            },
            Stroke::new(
                1.0,
                if is_report {
                    theme::ACCENT_BLUE()
                } else {
                    theme::BORDER()
                },
            ),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rep_rect.center(),
            Align2::CENTER_CENTER,
            "📋 Report",
            FontId::proportional(11.0),
            if is_report {
                theme::TEXT_PRIMARY()
            } else {
                theme::TEXT_MUTED()
            },
        );

        if quad_resp.clicked() && is_report {
            exam.report_view_mode = false;
            exam.current_subroute = ReportSubroute::QuadViewer;
            *action = ViewerAction::NavigateReportSubroute(ReportSubroute::QuadViewer);
        }
        if rep_resp.clicked() && !is_report {
            exam.report_view_mode = true;
            exam.current_subroute = ReportSubroute::ReportOverview;
            *action = ViewerAction::NavigateReportSubroute(ReportSubroute::ReportOverview);
        }

        // Contextual Viewport Controls (rendered ONLY in Quad View Mode)
        if !is_report {
            // Vertical Divider
            ui.painter().line_segment(
                [
                    Pos2::new(cur_x, mid_y - 10.0),
                    Pos2::new(cur_x, mid_y + 10.0),
                ],
                Stroke::new(1.0, theme::BORDER()),
            );
            cur_x += 12.0;

            // Eye Laterality Toggle [OD] / [OS]
            let od_rect =
                Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(48.0, 26.0));
            let os_rect =
                Rect::from_min_size(Pos2::new(cur_x + 52.0, mid_y - 13.0), Vec2::new(46.0, 26.0));
            cur_x += 104.0;

            let od_resp = ui.interact(od_rect, Id::new("lat_od_btn"), Sense::click());
            let os_resp = ui.interact(os_rect, Id::new("lat_os_btn"), Sense::click());

            let is_od = exam.laterality == EyeLaterality::OD;
            let is_os = exam.laterality == EyeLaterality::OS;

            ui.painter().rect(
                od_rect,
                CornerRadius::same(6),
                if is_od {
                    theme::CARD_HOVER()
                } else {
                    theme::INNER_TILE()
                },
                Stroke::new(
                    1.0,
                    if is_od {
                        theme::ACCENT_BLUE()
                    } else {
                        theme::BORDER()
                    },
                ),
                StrokeKind::Inside,
            );
            ui.painter().text(
                od_rect.center(),
                Align2::CENTER_CENTER,
                "OD",
                FontId::proportional(11.0),
                if is_od {
                    theme::TEXT_PRIMARY()
                } else {
                    theme::TEXT_MUTED()
                },
            );

            ui.painter().rect(
                os_rect,
                CornerRadius::same(6),
                if is_os {
                    theme::CARD_HOVER()
                } else {
                    theme::INNER_TILE()
                },
                Stroke::new(
                    1.0,
                    if is_os {
                        theme::ACCENT_BLUE()
                    } else {
                        theme::BORDER()
                    },
                ),
                StrokeKind::Inside,
            );
            ui.painter().text(
                os_rect.center(),
                Align2::CENTER_CENTER,
                "OS",
                FontId::proportional(11.0),
                if is_os {
                    theme::TEXT_PRIMARY()
                } else {
                    theme::TEXT_MUTED()
                },
            );

            if od_resp.clicked() && !is_od {
                exam.laterality = EyeLaterality::OD;
                *action = ViewerAction::LateralityChanged(EyeLaterality::OD);
            }
            if os_resp.clicked() && !is_os {
                exam.laterality = EyeLaterality::OS;
                *action = ViewerAction::LateralityChanged(EyeLaterality::OS);
            }

            // Vertical Divider
            ui.painter().line_segment(
                [
                    Pos2::new(cur_x, mid_y - 10.0),
                    Pos2::new(cur_x, mid_y + 10.0),
                ],
                Stroke::new(1.0, theme::BORDER()),
            );
            cur_x += 12.0;

            let is_synced = exam.sync_mode == SyncMode::Synchronized;
            let sync_rect =
                Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(98.0, 26.0));
            cur_x += 104.0;
            let sync_resp = ui.interact(sync_rect, Id::new("sync_mode_toggle"), Sense::click());
            ui.painter().rect(
                sync_rect,
                CornerRadius::same(6),
                if is_synced {
                    theme::CARD_HOVER()
                } else {
                    theme::INNER_TILE()
                },
                Stroke::new(
                    1.0,
                    if is_synced {
                        theme::ACCENT_BLUE()
                    } else {
                        theme::BORDER()
                    },
                ),
                StrokeKind::Inside,
            );
            ui.painter().text(
                sync_rect.center(),
                Align2::CENTER_CENTER,
                if is_synced { "⚡ Sync" } else { "⛌ Indep" },
                FontId::proportional(11.0),
                if is_synced {
                    theme::TEXT_PRIMARY()
                } else {
                    theme::TEXT_SECONDARY()
                },
            );
            if sync_resp.clicked() {
                let new_mode = if is_synced {
                    SyncMode::Independent
                } else {
                    SyncMode::Synchronized
                };
                exam.sync_mode = new_mode;
                *action = ViewerAction::SyncModeChanged(new_mode);
            }

            let fit_rect =
                Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(44.0, 26.0));
            let reset_rect =
                Rect::from_min_size(Pos2::new(cur_x + 48.0, mid_y - 13.0), Vec2::new(54.0, 26.0));
            cur_x += 108.0;

            let fit_resp = ui.interact(fit_rect, Id::new("fit_all_btn"), Sense::click());
            if fit_resp.hovered() {
                ui.painter()
                    .rect_filled(fit_rect, CornerRadius::same(6), theme::CARD_HOVER());
            }
            ui.painter().rect_stroke(
                fit_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                fit_rect.center(),
                Align2::CENTER_CENTER,
                "Fit",
                FontId::proportional(11.0),
                theme::TEXT_SECONDARY(),
            );

            let reset_resp = ui.interact(reset_rect, Id::new("reset_all_btn"), Sense::click());
            if reset_resp.hovered() {
                ui.painter()
                    .rect_filled(reset_rect, CornerRadius::same(6), theme::CARD_HOVER());
            }
            ui.painter().rect_stroke(
                reset_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                reset_rect.center(),
                Align2::CENTER_CENTER,
                "↺ Reset",
                FontId::proportional(11.0),
                theme::TEXT_SECONDARY(),
            );

            if fit_resp.clicked() {
                for state in &mut exam.panel_states {
                    state.zoom = 1.0;
                    state.pan = [0.0, 0.0];
                }
                *action = ViewerAction::FitAll;
            }
            if reset_resp.clicked() {
                for state in &mut exam.panel_states {
                    state.reset();
                }
                *action = ViewerAction::ResetAll;
            }
        }

        // Vertical Divider before Workflow Routes
        ui.painter().line_segment(
            [
                Pos2::new(cur_x, mid_y - 10.0),
                Pos2::new(cur_x, mid_y + 10.0),
            ],
            Stroke::new(1.0, theme::BORDER()),
        );
        cur_x += 12.0;

        // Distinct Routed Workflow Buttons: [📄 PDF Export], [📦 Session Dossier], [📋 Audit History]
        let pdf_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(82.0, 26.0));
        let sess_rect = Rect::from_min_size(
            Pos2::new(cur_x + 86.0, mid_y - 13.0),
            Vec2::new(116.0, 26.0),
        );
        let audit_rect = Rect::from_min_size(
            Pos2::new(cur_x + 206.0, mid_y - 13.0),
            Vec2::new(98.0, 26.0),
        );
        let _ = cur_x;

        let pdf_resp = ui.interact(pdf_rect, Id::new("toolbar_pdf_export_btn"), Sense::click());
        if pdf_resp.hovered() {
            ui.painter()
                .rect_filled(pdf_rect, CornerRadius::same(6), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            pdf_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            pdf_rect.center(),
            Align2::CENTER_CENTER,
            "📄 PDF Export",
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );
        if pdf_resp.clicked() {
            exam.current_subroute = ReportSubroute::PdfExport;
            *action = ViewerAction::NavigateReportSubroute(ReportSubroute::PdfExport);
        }

        let sess_resp = ui.interact(
            sess_rect,
            Id::new("toolbar_session_export_btn"),
            Sense::click(),
        );
        if sess_resp.hovered() {
            ui.painter()
                .rect_filled(sess_rect, CornerRadius::same(6), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            sess_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            sess_rect.center(),
            Align2::CENTER_CENTER,
            "📦 Session Dossier",
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );
        if sess_resp.clicked() {
            exam.current_subroute = ReportSubroute::SessionExport;
            *action = ViewerAction::NavigateReportSubroute(ReportSubroute::SessionExport);
        }

        let audit_resp = ui.interact(audit_rect, Id::new("toolbar_audit_btn"), Sense::click());
        if audit_resp.hovered() {
            ui.painter()
                .rect_filled(audit_rect, CornerRadius::same(6), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            audit_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            audit_rect.center(),
            Align2::CENTER_CENTER,
            "📋 Audit History",
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );
        if audit_resp.clicked() {
            exam.current_subroute = ReportSubroute::AuditHistory;
            *action = ViewerAction::NavigateReportSubroute(ReportSubroute::AuditHistory);
        }

        return;
    }

    // Right Side: Upload Button, Metadata, Run Stage I Button, and Readiness Pill
    let upload_btn_w = 144.0;
    let upload_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - upload_btn_w - 10.0, mid_y - 14.0),
        Vec2::new(upload_btn_w, 28.0),
    );
    if flags.upload {
        let upload_resp = ui.interact(upload_rect, Id::new("upload_fundus_main"), Sense::click());
        let up_hovered = upload_resp.hovered();

        ui.painter().rect(
            upload_rect,
            CornerRadius::same(6),
            if up_hovered {
                theme::CARD_HOVER()
            } else {
                theme::CARD_BG()
            },
            Stroke::new(
                1.0,
                if up_hovered {
                    theme::BORDER_LIGHT()
                } else {
                    theme::BORDER()
                },
            ),
            StrokeKind::Inside,
        );
        ui.painter().text(
            upload_rect.center(),
            Align2::CENTER_CENTER,
            "⭡ Upload Retinal Image",
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );
        if upload_resp.clicked() {
            *action = ViewerAction::UploadImageRequested;
        }
    }

    let mut right_cursor = if flags.upload {
        upload_rect.min.x - 8.0
    } else {
        rect.max.x - 10.0
    };

    // File metadata badge
    if let (true, Some(name), Some(dims)) = (flags.meta, uploaded_filename, uploaded_dimensions) {
        let meta_w = 145.0;
        let meta_rect = Rect::from_min_size(
            Pos2::new(right_cursor - meta_w, mid_y - 12.0),
            Vec2::new(meta_w, 24.0),
        );
        right_cursor -= meta_w + 8.0;

        ui.painter().rect(
            meta_rect,
            CornerRadius::same(5),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        let clean_name = if name.len() > 12 {
            format!("{}...", &name[..9])
        } else {
            name.to_string()
        };
        ui.painter().text(
            meta_rect.center(),
            Align2::CENTER_CENTER,
            format!("{} • {}x{}", clean_name, dims[0], dims[1]),
            FontId::proportional(10.0),
            theme::TEXT_MUTED(),
        );
    }

    // Run / Cancel (when an image is loaded). The cascade state wins over the Stage I-only state.
    if has_image {
        let cascade_phase = exam.cascade.phase;
        let cascade_available = exam.cascade.available;
        let finishing_previous = exam.cascade.finishing_previous;
        let stage1_only_ready = !cascade_available && backend_readiness.is_ready();
        let stage1_only_busy = !cascade_available && backend_readiness.is_processing();
        // (label, fill, stroke, id, action on click, enabled)
        let button: Option<(&str, Color32, Color32, &str, ViewerAction, bool)> =
            if cascade_phase != crate::engine::cascade::CascadePhase::Idle {
                let cancelling = cascade_phase == crate::engine::cascade::CascadePhase::Cancelling;
                Some((
                    if cancelling {
                        "Cancelling..."
                    } else {
                        "✕ Cancel Analysis"
                    },
                    Color32::from_rgb(127, 29, 29),
                    theme::DIR_ERROR(),
                    "cancel_analysis_btn",
                    ViewerAction::CancelInference,
                    !cancelling,
                ))
            } else if cascade_available {
                Some((
                    if finishing_previous {
                        "Finishing previous run..."
                    } else {
                        "▶ Run Full Analysis"
                    },
                    Color32::from_rgb(20, 83, 45),
                    theme::DIR_SUCCESS(),
                    "run_full_analysis_btn",
                    ViewerAction::RunStage1Inference,
                    !finishing_previous,
                ))
            } else if stage1_only_ready {
                Some((
                    "▶ Run Stage I Screening",
                    Color32::from_rgb(20, 83, 45),
                    theme::DIR_SUCCESS(),
                    "run_stage1_screening_btn",
                    ViewerAction::RunStage1Inference,
                    true,
                ))
            } else if stage1_only_busy {
                Some((
                    "✕ Cancel Screening",
                    Color32::from_rgb(127, 29, 29),
                    theme::DIR_ERROR(),
                    "cancel_stage1_screening_btn",
                    ViewerAction::CancelInference,
                    true,
                ))
            } else {
                None
            };
        if let Some((label, fill, stroke, id, on_click, enabled)) = button {
            let w = if label.len() > 22 { 185.0 } else { 160.0 };
            let rect = Rect::from_min_size(
                Pos2::new(right_cursor - w, mid_y - 14.0),
                Vec2::new(w, 28.0),
            );
            right_cursor -= w + 8.0;
            let resp = ui.interact(
                rect,
                Id::new(id),
                if enabled {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            let fill = if !enabled {
                theme::INNER_TILE()
            } else if resp.hovered() {
                fill.gamma_multiply(1.25)
            } else {
                fill
            };
            ui.painter().rect(
                rect,
                CornerRadius::same(6),
                fill,
                Stroke::new(1.0, if enabled { stroke } else { theme::BORDER() }),
                StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(11.0),
                if enabled {
                    Color32::WHITE
                } else {
                    theme::TEXT_MUTED()
                },
            );
            if enabled && resp.clicked() {
                *action = on_click;
            }
        }
    }

    if flags.pill {
        // Backend Readiness Pill
        let pill_w = 145.0;
        let pill_rect = Rect::from_min_size(
            Pos2::new(right_cursor - pill_w, mid_y - 12.0),
            Vec2::new(pill_w, 24.0),
        );

        let cascade_active = exam.cascade.is_active();
        let (dot_color, pill_bg, pill_stroke, status_label) = match backend_readiness {
            _ if cascade_active => (
                theme::ACCENT_BLUE(),
                Color32::from_rgb(16, 28, 48),
                Stroke::new(1.0, theme::ACCENT_BLUE()),
                "Analysis Active...".to_string(),
            ),
            crate::engine::BackendReadiness::Ready { .. } if exam.cascade.available => (
                theme::DIR_SUCCESS(),
                Color32::from_rgb(10, 30, 20),
                Stroke::new(1.0, theme::DIR_SUCCESS()),
                "Full Analysis Ready (CPU)".to_string(),
            ),
            crate::engine::BackendReadiness::Ready { provider, .. } => (
                theme::DIR_SUCCESS(),
                Color32::from_rgb(10, 30, 20),
                Stroke::new(1.0, theme::DIR_SUCCESS()),
                format!("Stage I Ready ({provider})"),
            ),
            crate::engine::BackendReadiness::Processing { .. } => (
                theme::ACCENT_BLUE(),
                Color32::from_rgb(16, 28, 48),
                Stroke::new(1.0, theme::ACCENT_BLUE()),
                "Screening Active...".to_string(),
            ),
            crate::engine::BackendReadiness::Cancelling { .. } => (
                theme::DIR_AMBER(),
                theme::DIR_AMBER_BG(),
                Stroke::new(1.0, theme::DIR_AMBER()),
                "Cancelling...".to_string(),
            ),
            crate::engine::BackendReadiness::ModelMissing { .. } => (
                theme::DIR_AMBER(),
                theme::DIR_AMBER_BG(),
                Stroke::new(1.0, theme::DIR_AMBER()),
                "Stage I: Missing".to_string(),
            ),
            crate::engine::BackendReadiness::Unavailable { .. } => (
                theme::DIR_AMBER(),
                theme::DIR_AMBER_BG(),
                Stroke::new(1.0, theme::DIR_AMBER()),
                "Stage I: Unavailable".to_string(),
            ),
            crate::engine::BackendReadiness::Failed { .. } => (
                theme::DIR_ERROR(),
                Color32::from_rgb(40, 16, 16),
                Stroke::new(1.0, theme::DIR_ERROR()),
                "Stage I: Failed".to_string(),
            ),
            _ => (
                Color32::from_rgb(156, 163, 175),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                "Stage I: Initializing".to_string(),
            ),
        };

        ui.painter().rect(
            pill_rect,
            CornerRadius::same(5),
            pill_bg,
            pill_stroke,
            StrokeKind::Inside,
        );
        ui.painter().circle_filled(
            Pos2::new(pill_rect.min.x + 12.0, pill_rect.center().y),
            3.5,
            dot_color,
        );
        ui.painter().text(
            Pos2::new(pill_rect.min.x + 22.0, pill_rect.center().y),
            Align2::LEFT_CENTER,
            status_label,
            FontId::proportional(10.5),
            theme::TEXT_PRIMARY(),
        );
    }
}

/// Renders the authentic Stage I Clinical Screening Card
fn render_stage1_result_card(
    ui: &mut Ui,
    rect: Rect,
    res: &crate::engine::Stage1ScreeningResult,
    action: &mut ViewerAction,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        Color32::from_rgb(15, 20, 26),
        Stroke::new(1.0, Color32::from_rgb(51, 65, 85)),
        StrokeKind::Inside,
    );

    let is_negative = res.decision == crate::engine::Stage1GateDecision::NegativeRoute;
    let badge_color = if is_negative {
        Color32::from_rgb(16, 185, 129)
    } else {
        theme::DIR_AMBER()
    };
    let badge_bg = if is_negative {
        Color32::from_rgb(6, 78, 59)
    } else {
        theme::DIR_AMBER_BG()
    };
    let badge_text = if is_negative {
        "✓ NEGATIVE ROUTE (DR GRADE 0 TRIAGE)"
    } else {
        "⚠ CONTINUE CASCADE (DR REFERRAL / GRADES 1–5)"
    };

    let line1_y = rect.min.y + 18.0;
    let mut cur_x = rect.min.x + 14.0;

    // Decision Badge
    let badge_w = 275.0;
    let badge_rect =
        Rect::from_min_size(Pos2::new(cur_x, line1_y - 10.0), Vec2::new(badge_w, 20.0));
    ui.painter().rect(
        badge_rect,
        CornerRadius::same(4),
        badge_bg,
        Stroke::new(1.0, badge_color),
        StrokeKind::Inside,
    );
    ui.painter().text(
        badge_rect.center(),
        Align2::CENTER_CENTER,
        badge_text,
        FontId::proportional(10.5),
        Color32::WHITE,
    );
    cur_x += badge_w + 16.0;

    // Score & Threshold
    ui.painter().text(
        Pos2::new(cur_x, line1_y),
        Align2::LEFT_CENTER,
        format!(
            "Score: {:.4} (Threshold: {:.4})",
            res.screening_score, res.threshold
        ),
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    cur_x += 210.0;

    // Raw logit
    ui.painter().text(
        Pos2::new(cur_x, line1_y),
        Align2::LEFT_CENTER,
        format!("Logit: {:+.4}", res.raw_logit),
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
    cur_x += 105.0;

    // Elapsed
    ui.painter().text(
        Pos2::new(cur_x, line1_y),
        Align2::LEFT_CENTER,
        format!(
            "Inference: {}ms (Prep: {}ms)",
            res.elapsed_ms, res.preprocessing_ms
        ),
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );

    // Dismiss Button
    let close_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - 26.0, rect.min.y + 8.0),
        Vec2::new(18.0, 18.0),
    );
    let close_resp = ui.interact(close_rect, Id::new("dismiss_stage1_card"), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(3), theme::CARD_HOVER());
    }
    ui.painter().text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );
    if close_resp.clicked() {
        *action = ViewerAction::DismissResult;
    }

    // Line 2: Model Provenance and Mandatory Non-Diagnostic Caveat
    let line2_y = rect.min.y + 45.0;
    let prov_text = format!(
        "Model: {} v{} • {}",
        res.model_id, res.model_version, res.provider
    );
    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, line2_y),
        Align2::LEFT_CENTER,
        prov_text,
        FontId::proportional(10.0),
        theme::TEXT_MUTED(),
    );

    let caveat = "* Research screening prototype only. Not certified for clinical diagnosis. Clinical examination required for definitive care.";
    ui.painter().text(
        Pos2::new(rect.max.x - 30.0, line2_y),
        Align2::RIGHT_CENTER,
        caveat,
        FontId::proportional(9.5),
        Color32::from_rgb(148, 163, 184),
    );
}

/// Renders an error notice card when Stage I inference fails
fn render_stage1_error_card(ui: &mut Ui, rect: Rect, err: &str, action: &mut ViewerAction) {
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        Color32::from_rgb(45, 18, 18),
        Stroke::new(1.0, theme::DIR_ERROR()),
        StrokeKind::Inside,
    );

    let mid_y = rect.center().y;
    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, mid_y),
        Align2::LEFT_CENTER,
        format!("⚠ Stage I Inference Error: {err}"),
        FontId::proportional(11.0),
        theme::DIR_ERROR(),
    );

    let close_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - 26.0, mid_y - 9.0),
        Vec2::new(18.0, 18.0),
    );
    let close_resp = ui.interact(close_rect, Id::new("dismiss_stage1_err"), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(3), theme::CARD_HOVER());
    }
    ui.painter().text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(11.0),
        Color32::WHITE,
    );
    if close_resp.clicked() {
        *action = ViewerAction::DismissResult;
    }
}

/// Renders an individual diagnostic viewport panel (Original, Lesions, Vessels, Explainability)
#[allow(clippy::too_many_arguments)]
fn render_single_panel(
    ui: &mut Ui,
    ctx: &egui::Context,
    panel_rect: Rect,
    panel_id: ViewPanelId,
    exam: &mut RetinalExaminationView,
    mode_textures: &[Option<TextureHandle>; 4],
    uploaded_dimensions: Option<[usize; 2]>,
    is_expanded: bool,
    action: &mut ViewerAction,
) {
    let p_idx = panel_id as usize;

    // Panel Outer Frame
    ui.painter().rect(
        panel_rect,
        CornerRadius::same(12),
        theme::STAGE_MATTE_BLACK(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    // Panel Header Bar (32 px)
    let header_h = 32.0;
    let header_rect = Rect::from_min_size(panel_rect.min, Vec2::new(panel_rect.width(), header_h));
    ui.painter().rect(
        header_rect,
        CornerRadius::same(12),
        theme::CARD_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let title_x = header_rect.min.x + 12.0;
    let title_y = header_rect.center().y;

    // Panel Title
    ui.painter().text(
        Pos2::new(title_x, title_y),
        Align2::LEFT_CENTER,
        panel_id.title(),
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );

    // Maximize / Fullscreen button on far right
    let max_btn_w = 26.0;
    let max_btn_rect = Rect::from_min_size(
        Pos2::new(header_rect.max.x - max_btn_w - 6.0, header_rect.min.y + 4.0),
        Vec2::new(max_btn_w, 24.0),
    );
    let max_resp = ui.interact(
        max_btn_rect,
        Id::new("panel_max_btn").with(p_idx),
        Sense::click(),
    );
    if max_resp.hovered() {
        ui.painter()
            .rect_filled(max_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
    }
    ui.painter().text(
        max_btn_rect.center(),
        Align2::CENTER_CENTER,
        if is_expanded { "⤡" } else { "⤢" },
        FontId::proportional(13.0),
        theme::TEXT_SECONDARY(),
    );
    if max_resp.clicked() {
        if is_expanded {
            exam.expanded_panel = None;
            *action = ViewerAction::ExpandPanel(None);
        } else {
            exam.expanded_panel = Some(panel_id);
            *action = ViewerAction::ExpandPanel(Some(panel_id));
        }
    }

    // Panel-specific Header Badges and Toggles
    render_panel_header_controls(ui, header_rect, panel_id, exam, max_btn_rect.min.x);

    // Bottom HUD area (28 px)
    let hud_h = 28.0;
    let hud_rect = Rect::from_min_size(
        Pos2::new(panel_rect.min.x, panel_rect.max.y - hud_h),
        Vec2::new(panel_rect.width(), hud_h),
    );

    // Viewport Stage Rect (occupies space between Header and HUD)
    let stage_rect = Rect::from_min_max(
        Pos2::new(panel_rect.min.x, header_rect.max.y),
        Pos2::new(panel_rect.max.x, hud_rect.min.y),
    );

    let stage_resp = ui.interact(
        stage_rect,
        Id::new("panel_stage_interact").with(p_idx),
        Sense::click_and_drag(),
    );

    // Coordinate inspection & Pan/Zoom interaction when texture is present
    let raw_tex = &mode_textures[0];
    if let Some(texture) = raw_tex {
        let (img_w, img_h) = if let Some(dims) = uploaded_dimensions {
            (dims[0] as f32, dims[1] as f32)
        } else {
            (texture.size_vec2().x, texture.size_vec2().y)
        };

        let current_zoom = exam.panel_states[p_idx].zoom;
        let current_pan = Vec2::new(
            exam.panel_states[p_idx].pan[0],
            exam.panel_states[p_idx].pan[1],
        );

        let (fit_base, display_rect) =
            compute_display_rect(stage_rect, img_w, img_h, current_zoom, current_pan);

        // Hover coordinate tracking
        if let Some(hover_pos) = stage_resp.hover_pos() {
            if let Some((pixel, norm)) =
                screen_to_image_coords(hover_pos, display_rect, img_w, img_h)
            {
                exam.panel_states[p_idx].hovered_pixel = Some(pixel);
                exam.panel_states[p_idx].hovered_norm = Some(norm);
            } else {
                exam.panel_states[p_idx].hovered_pixel = None;
                exam.panel_states[p_idx].hovered_norm = None;
            }
        } else {
            exam.panel_states[p_idx].hovered_pixel = None;
            exam.panel_states[p_idx].hovered_norm = None;
        }

        // Mouse Wheel Zoom (cursor-centered)
        if stage_resp.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.1 {
                let zoom_mult = if scroll_delta > 0.0 { 1.12 } else { 0.89 };
                let pointer_pos = stage_resp
                    .hover_pos()
                    .unwrap_or_else(|| stage_rect.center());
                let (new_zoom, new_pan) = cursor_centered_zoom(
                    pointer_pos,
                    zoom_mult,
                    current_zoom,
                    current_pan,
                    fit_base,
                    0.25,
                    10.0,
                );

                if exam.sync_mode == SyncMode::Synchronized {
                    for state in &mut exam.panel_states {
                        state.zoom = new_zoom;
                        state.pan = [new_pan.x, new_pan.y];
                    }
                    *action = ViewerAction::SyncZoomAndPan {
                        zoom: new_zoom,
                        pan: [new_pan.x, new_pan.y],
                    };
                } else {
                    exam.panel_states[p_idx].zoom = new_zoom;
                    exam.panel_states[p_idx].pan = [new_pan.x, new_pan.y];
                }
                ctx.request_repaint();
            }
        }

        // Drag to Pan
        if stage_resp.dragged() {
            let delta = stage_resp.drag_delta();
            let new_pan = current_pan + delta;

            if exam.sync_mode == SyncMode::Synchronized {
                for state in &mut exam.panel_states {
                    state.pan = [new_pan.x, new_pan.y];
                }
                *action = ViewerAction::SyncZoomAndPan {
                    zoom: current_zoom,
                    pan: [new_pan.x, new_pan.y],
                };
            } else {
                exam.panel_states[p_idx].pan = [new_pan.x, new_pan.y];
            }
            ctx.request_repaint();
        }

        // Double click to reset
        if stage_resp.double_clicked() {
            if exam.sync_mode == SyncMode::Synchronized {
                for state in &mut exam.panel_states {
                    state.reset();
                }
                *action = ViewerAction::ResetAll;
            } else {
                exam.panel_states[p_idx].reset();
            }
            ctx.request_repaint();
        }

        // Clip rendering to stage viewport
        let stage_painter = ui.painter().with_clip_rect(stage_rect);

        // Render base retinal fundus image
        stage_painter.image(
            texture.id(),
            display_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // Render Panel-specific Overlays or Truthful Unavailable States
        match panel_id {
            ViewPanelId::Original => {
                // Pure authentic fundus view
            }
            ViewPanelId::Lesions => {
                render_lesion_overlay(&stage_painter, display_rect, &exam.lesion_artifact);
            }
            ViewPanelId::Vessels => {
                render_vessel_overlay(&stage_painter, display_rect, &exam.vessel_artifact);
            }
            ViewPanelId::Explainability => {
                render_explainability_overlay(
                    &stage_painter,
                    display_rect,
                    &exam.explainability_artifact,
                );
            }
        }

        // Hover crosshairs on image
        if let Some(hover_pos) = stage_resp.hover_pos() {
            if display_rect.contains(hover_pos) {
                let cross_color = Color32::from_rgba_unmultiplied(255, 255, 255, 75);
                let cross_stroke = Stroke::new(1.0, cross_color);
                stage_painter.line_segment(
                    [
                        Pos2::new(hover_pos.x - 8.0, hover_pos.y),
                        Pos2::new(hover_pos.x + 8.0, hover_pos.y),
                    ],
                    cross_stroke,
                );
                stage_painter.line_segment(
                    [
                        Pos2::new(hover_pos.x, hover_pos.y - 8.0),
                        Pos2::new(hover_pos.x, hover_pos.y + 8.0),
                    ],
                    cross_stroke,
                );
            }
        }
    } else {
        // Empty Stage Presentation
        render_empty_stage_placeholder(ui, stage_rect, panel_id);
    }

    // Render Bottom HUD (Coordinates & Zoom Controls)
    render_panel_hud(
        ui,
        ctx,
        hud_rect,
        p_idx,
        exam,
        mode_textures[0].is_some(),
        action,
    );
}

/// Small "open the detail page" button at the right edge of a Quad panel title bar.
fn detail_link(ui: &mut Ui, header_rect: Rect, right_x: &mut f32, id: &str, tip: &str) -> bool {
    let w = 24.0;
    let rect = Rect::from_min_size(
        Pos2::new(*right_x - w, header_rect.center().y - 11.0),
        Vec2::new(w, 22.0),
    );
    *right_x -= w + 6.0;
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        theme::INNER_TILE(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "↗",
        FontId::proportional(12.0),
        if resp.hovered() {
            theme::TEXT_PRIMARY()
        } else {
            theme::TEXT_SECONDARY()
        },
    );
    resp.on_hover_text(tip).clicked()
}

/// Renders panel-specific toggles and controls in the header bar
fn render_panel_header_controls(
    ui: &mut Ui,
    header_rect: Rect,
    panel_id: ViewPanelId,
    exam: &mut RetinalExaminationView,
    right_limit_x: f32,
) {
    let mid_y = header_rect.center().y;
    let mut right_x = right_limit_x - 8.0;

    match panel_id {
        ViewPanelId::Original => {
            // Laterality Badge
            let badge_w = 40.0;
            let badge_rect = Rect::from_min_size(
                Pos2::new(right_x - badge_w, mid_y - 11.0),
                Vec2::new(badge_w, 22.0),
            );
            ui.painter().rect(
                badge_rect,
                CornerRadius::same(4),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                badge_rect.center(),
                Align2::CENTER_CENTER,
                exam.laterality.short_code(),
                FontId::proportional(11.0),
                theme::ACCENT_BLUE(),
            );
        }
        ViewPanelId::Lesions => {
            if detail_link(
                ui,
                header_rect,
                &mut right_x,
                "quad_open_lesions",
                "Open Lesion Analysis",
            ) {
                exam.lesion_link_clicked = true;
            }
            // Opacity and category badges
            let op_text = format!("Opacity {:.0}%", exam.lesion_artifact.opacity * 100.0);
            let op_w = 80.0;
            let op_rect = Rect::from_min_size(
                Pos2::new(right_x - op_w, mid_y - 11.0),
                Vec2::new(op_w, 22.0),
            );
            right_x -= op_w + 6.0;

            let op_resp = ui.interact(op_rect, Id::new("lesion_op_btn"), Sense::click());
            if op_resp.clicked() {
                exam.lesion_artifact.opacity = if exam.lesion_artifact.opacity >= 0.9 {
                    0.3
                } else {
                    exam.lesion_artifact.opacity + 0.3
                };
            }
            ui.painter().rect(
                op_rect,
                CornerRadius::same(4),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                op_rect.center(),
                Align2::CENTER_CENTER,
                op_text,
                FontId::proportional(10.0),
                theme::TEXT_MUTED(),
            );

            // Category pills: MA (Microaneurysms), HE (Hemorrhages), EX (Exudates), CWS (Cotton-wool spots)
            let pills = [
                (
                    "CWS",
                    &mut exam.lesion_artifact.toggles.cotton_wool_spots,
                    Color32::from_rgb(6, 182, 212),
                ),
                (
                    "EX",
                    &mut exam.lesion_artifact.toggles.hard_exudates,
                    theme::DIR_AMBER(),
                ),
                (
                    "HE",
                    &mut exam.lesion_artifact.toggles.hemorrhages,
                    Color32::from_rgb(220, 38, 38),
                ),
                (
                    "MA",
                    &mut exam.lesion_artifact.toggles.microaneurysms,
                    theme::DIR_ERROR(),
                ),
            ];

            for (tag, active, col) in pills {
                let pill_w = 32.0;
                let pill_rect = Rect::from_min_size(
                    Pos2::new(right_x - pill_w, mid_y - 11.0),
                    Vec2::new(pill_w, 22.0),
                );
                right_x -= pill_w + 4.0;

                let pill_resp =
                    ui.interact(pill_rect, Id::new("lesion_pill").with(tag), Sense::click());
                if pill_resp.clicked() {
                    *active = !*active;
                }

                let is_on = *active;
                ui.painter().rect(
                    pill_rect,
                    CornerRadius::same(4),
                    if is_on {
                        theme::CARD_HOVER()
                    } else {
                        theme::INNER_TILE()
                    },
                    Stroke::new(1.0, if is_on { col } else { theme::BORDER() }),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    pill_rect.center(),
                    Align2::CENTER_CENTER,
                    tag,
                    FontId::proportional(9.5),
                    if is_on {
                        theme::TEXT_PRIMARY()
                    } else {
                        theme::TEXT_MUTED()
                    },
                );
            }
        }
        ViewPanelId::Vessels => {
            if detail_link(
                ui,
                header_rect,
                &mut right_x,
                "quad_open_vessels",
                "Open Vessel Analysis",
            ) {
                exam.vessel_link_clicked = true;
            }
            // Mode toggle: [Overlay] vs [Mask Only]
            let mode_w = 68.0;
            let mode_rect = Rect::from_min_size(
                Pos2::new(right_x - mode_w, mid_y - 11.0),
                Vec2::new(mode_w, 22.0),
            );
            right_x -= mode_w + 6.0;

            let mode_resp = ui.interact(mode_rect, Id::new("vessel_mode_toggle"), Sense::click());
            if mode_resp.clicked() {
                exam.vessel_artifact.mask_only = !exam.vessel_artifact.mask_only;
            }
            ui.painter().rect(
                mode_rect,
                CornerRadius::same(4),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                mode_rect.center(),
                Align2::CENTER_CENTER,
                if exam.vessel_artifact.mask_only {
                    "Mask Only"
                } else {
                    "Overlay"
                },
                FontId::proportional(10.0),
                theme::ACCENT_BLUE(),
            );

            // Opacity toggle
            let op_text = format!("Opacity {:.0}%", exam.vessel_artifact.opacity * 100.0);
            let op_w = 78.0;
            let op_rect = Rect::from_min_size(
                Pos2::new(right_x - op_w, mid_y - 11.0),
                Vec2::new(op_w, 22.0),
            );
            let op_resp = ui.interact(op_rect, Id::new("vessel_op_btn"), Sense::click());
            if op_resp.clicked() {
                exam.vessel_artifact.opacity = if exam.vessel_artifact.opacity >= 0.9 {
                    0.4
                } else {
                    exam.vessel_artifact.opacity + 0.3
                };
            }
            ui.painter().rect(
                op_rect,
                CornerRadius::same(4),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                op_rect.center(),
                Align2::CENTER_CENTER,
                op_text,
                FontId::proportional(10.0),
                theme::TEXT_MUTED(),
            );
        }
        ViewPanelId::Explainability => {
            // Attribution tag
            let tag_w = 110.0;
            let tag_rect = Rect::from_min_size(
                Pos2::new(right_x - tag_w, mid_y - 11.0),
                Vec2::new(tag_w, 22.0),
            );
            ui.painter().rect(
                tag_rect,
                CornerRadius::same(4),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );
            let resp = ui.interact(
                tag_rect,
                egui::Id::new("quad_open_explainability"),
                Sense::click(),
            );
            if resp.clicked() {
                exam.explain_link_clicked = true;
            }
            ui.painter().text(
                tag_rect.center(),
                Align2::CENTER_CENTER,
                "Open Explainability",
                FontId::proportional(10.0),
                if resp.hovered() {
                    theme::TEXT_PRIMARY()
                } else {
                    theme::TEXT_SECONDARY()
                },
            );
        }
    }
}

/// Renders bottom HUD inside each panel: discrete coordinates, zoom controls, reset
fn render_panel_hud(
    ui: &mut Ui,
    ctx: &egui::Context,
    hud_rect: Rect,
    p_idx: usize,
    exam: &mut RetinalExaminationView,
    has_image: bool,
    action: &mut ViewerAction,
) {
    ui.painter().rect(
        hud_rect,
        CornerRadius::same(6),
        theme::INNER_TILE(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let mid_y = hud_rect.center().y;

    // Left side: Coordinate readout
    let coord_text = if !has_image {
        "⌖ Awaiting image".to_string()
    } else if let (Some(px), Some(norm)) = (
        exam.panel_states[p_idx].hovered_pixel,
        exam.panel_states[p_idx].hovered_norm,
    ) {
        format!(
            "⌖ X: {:<4} Y: {:<4}  •  u: {:.3}  v: {:.3}",
            px.0, px.1, norm.0, norm.1
        )
    } else {
        "⌖ Hover image to inspect coordinates".to_string()
    };

    ui.painter().text(
        Pos2::new(hud_rect.min.x + 10.0, mid_y),
        Align2::LEFT_CENTER,
        coord_text,
        FontId::monospace(10.0),
        theme::TEXT_MUTED(),
    );

    // Right side: Zoom controls [ − ] [ 100% ] [ + ] [ ↺ ]
    let zoom_val = exam.panel_states[p_idx].zoom;
    let mut btn_x = hud_rect.max.x - 30.0;

    // Reset [↺]
    let reset_rect = Rect::from_min_size(Pos2::new(btn_x, mid_y - 10.0), Vec2::new(24.0, 20.0));
    let reset_resp = ui.interact(reset_rect, Id::new("hud_reset").with(p_idx), Sense::click());
    if reset_resp.hovered() {
        ui.painter()
            .rect_filled(reset_rect, CornerRadius::same(3), theme::CARD_HOVER());
    }
    ui.painter().text(
        reset_rect.center(),
        Align2::CENTER_CENTER,
        "↺",
        FontId::proportional(12.0),
        theme::TEXT_SECONDARY(),
    );
    if reset_resp.clicked() {
        if exam.sync_mode == SyncMode::Synchronized {
            for state in &mut exam.panel_states {
                state.reset();
            }
            *action = ViewerAction::ResetAll;
        } else {
            exam.panel_states[p_idx].reset();
        }
        ctx.request_repaint();
    }
    btn_x -= 30.0;

    // Zoom In [+]
    let zin_rect = Rect::from_min_size(Pos2::new(btn_x, mid_y - 10.0), Vec2::new(24.0, 20.0));
    let zin_resp = ui.interact(zin_rect, Id::new("hud_zin").with(p_idx), Sense::click());
    if zin_resp.hovered() {
        ui.painter()
            .rect_filled(zin_rect, CornerRadius::same(3), theme::CARD_HOVER());
    }
    ui.painter().text(
        zin_rect.center(),
        Align2::CENTER_CENTER,
        "+",
        FontId::proportional(13.0),
        theme::TEXT_PRIMARY(),
    );
    if zin_resp.clicked() {
        let new_zoom = (zoom_val * 1.15).min(10.0);
        if exam.sync_mode == SyncMode::Synchronized {
            for state in &mut exam.panel_states {
                state.zoom = new_zoom;
            }
        } else {
            exam.panel_states[p_idx].zoom = new_zoom;
        }
        ctx.request_repaint();
    }
    btn_x -= 50.0;

    // Zoom Label
    let zlab_rect = Rect::from_min_size(Pos2::new(btn_x, mid_y - 10.0), Vec2::new(46.0, 20.0));
    ui.painter().text(
        zlab_rect.center(),
        Align2::CENTER_CENTER,
        format!("{:.0}%", zoom_val * 100.0),
        FontId::proportional(10.5),
        theme::TEXT_PRIMARY(),
    );
    btn_x -= 28.0;

    // Zoom Out [−]
    let zout_rect = Rect::from_min_size(Pos2::new(btn_x, mid_y - 10.0), Vec2::new(24.0, 20.0));
    let zout_resp = ui.interact(zout_rect, Id::new("hud_zout").with(p_idx), Sense::click());
    if zout_resp.hovered() {
        ui.painter()
            .rect_filled(zout_rect, CornerRadius::same(3), theme::CARD_HOVER());
    }
    ui.painter().text(
        zout_rect.center(),
        Align2::CENTER_CENTER,
        "−",
        FontId::proportional(13.0),
        theme::TEXT_PRIMARY(),
    );
    if zout_resp.clicked() {
        let new_zoom = (zoom_val * 0.85).max(0.25);
        if exam.sync_mode == SyncMode::Synchronized {
            for state in &mut exam.panel_states {
                state.zoom = new_zoom;
            }
        } else {
            exam.panel_states[p_idx].zoom = new_zoom;
        }
        ctx.request_repaint();
    }
}

/// Renders empty placeholder or truthful "Awaiting Freebuff Inference" card
fn render_empty_stage_placeholder(ui: &mut Ui, stage_rect: Rect, panel_id: ViewPanelId) {
    let center = stage_rect.center();

    if panel_id == ViewPanelId::Original {
        // Original panel upload prompt
        let card_w = 280.0;
        let card_h = 100.0;
        let card_rect = Rect::from_center_size(center, Vec2::new(card_w, card_h));

        ui.painter().rect(
            card_rect,
            CornerRadius::same(10),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );

        ui.painter().circle_stroke(
            Pos2::new(center.x, center.y - 16.0),
            14.0,
            Stroke::new(1.2, theme::BORDER_LIGHT()),
        );
        ui.painter().text(
            Pos2::new(center.x, center.y - 16.0),
            Align2::CENTER_CENTER,
            "⭡",
            FontId::proportional(14.0),
            theme::TEXT_SECONDARY(),
        );

        ui.painter().text(
            Pos2::new(center.x, center.y + 12.0),
            Align2::CENTER_CENTER,
            "No Retinal Image Loaded",
            FontId::proportional(12.0),
            theme::TEXT_PRIMARY(),
        );
        ui.painter().text(
            Pos2::new(center.x, center.y + 28.0),
            Align2::CENTER_CENTER,
            "Click 'Upload Retinal Image' in top toolbar",
            FontId::proportional(10.0),
            theme::TEXT_MUTED(),
        );
    } else {
        // Panels 1, 2, 3 truthful backend status card
        let card_w = 260.0;
        let card_h = 90.0;
        let card_rect = Rect::from_center_size(center, Vec2::new(card_w, card_h));

        ui.painter().rect(
            card_rect,
            CornerRadius::same(8),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );

        let (stage_title, model_id) = match panel_id {
            ViewPanelId::Lesions => ("Awaiting Lesion Inference", "HSMoE-AUNet Segmentation"),
            ViewPanelId::Vessels => ("Awaiting Vessel Segmentation", "SwinHRUNet++ Caliber"),
            ViewPanelId::Explainability => {
                ("Awaiting Attribution Heatmap", "HierarchiRetina Grad-CAM")
            }
            _ => ("Awaiting Backend Output", "Codex Medical Engine"),
        };

        ui.painter().circle_stroke(
            Pos2::new(center.x, center.y - 16.0),
            12.0,
            Stroke::new(1.0, theme::BORDER()),
        );
        ui.painter().text(
            Pos2::new(center.x, center.y - 16.0),
            Align2::CENTER_CENTER,
            "⏳",
            FontId::proportional(11.0),
            theme::TEXT_MUTED(),
        );

        ui.painter().text(
            Pos2::new(center.x, center.y + 10.0),
            Align2::CENTER_CENTER,
            stage_title,
            FontId::proportional(11.5),
            theme::TEXT_SECONDARY(),
        );
        ui.painter().text(
            Pos2::new(center.x, center.y + 26.0),
            Align2::CENTER_CENTER,
            format!("Requires {model_id}"),
            FontId::proportional(9.5),
            theme::TEXT_MUTED(),
        );
    }
}

/// Renders lesion segmentation overlay: real backend masks when verified, otherwise a truthful notice.
pub(crate) fn render_lesion_overlay(
    painter: &egui::Painter,
    display_rect: Rect,
    artifact: &crate::domain::LesionMaskArtifact,
) {
    if artifact.status == ArtifactLoadStatus::ArtifactReady {
        let tint = Color32::from_white_alpha((artifact.opacity.clamp(0.0, 1.0) * 255.0) as u8);
        let layers = [
            (artifact.toggles.microaneurysms, &artifact.textures.ma),
            (artifact.toggles.hemorrhages, &artifact.textures.he),
            (artifact.toggles.hard_exudates, &artifact.textures.ex),
            (artifact.toggles.cotton_wool_spots, &artifact.textures.cws),
        ];
        for (enabled, layer) in layers {
            if let (true, Some(mask)) = (enabled, layer) {
                painter.image(
                    mask.texture.id(),
                    display_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    tint,
                );
            }
        }
        return;
    }
    {
        // Truthful notice badge inside viewport
        let badge_rect = Rect::from_center_size(display_rect.center(), Vec2::new(240.0, 48.0));
        painter.rect(
            badge_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(16, 17, 19, 210),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y - 6.0),
            Align2::CENTER_CENTER,
            "No lesion masks for this image",
            FontId::proportional(11.0),
            theme::TEXT_SECONDARY(),
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y + 10.0),
            Align2::CENTER_CENTER,
            "Run Full Analysis • no markers are drawn without a result",
            FontId::proportional(9.0),
            theme::TEXT_MUTED(),
        );
    }
}

/// Renders vessel segmentation overlay: the real SwinHRUNet++ mask when verified.
pub(crate) fn render_vessel_overlay(
    painter: &egui::Painter,
    display_rect: Rect,
    artifact: &crate::domain::VesselMaskArtifact,
) {
    if artifact.status == ArtifactLoadStatus::ArtifactReady {
        if let Some(mask) = &artifact.texture {
            if artifact.mask_only {
                painter.rect_filled(display_rect, 0.0, Color32::BLACK);
            }
            let tint = Color32::from_white_alpha((artifact.opacity.clamp(0.0, 1.0) * 255.0) as u8);
            painter.image(
                mask.texture.id(),
                display_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        }
        return;
    }
    {
        let badge_rect = Rect::from_center_size(display_rect.center(), Vec2::new(240.0, 48.0));
        painter.rect(
            badge_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(16, 17, 19, 210),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y - 6.0),
            Align2::CENTER_CENTER,
            "No vessel mask for this image",
            FontId::proportional(11.0),
            theme::TEXT_SECONDARY(),
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y + 10.0),
            Align2::CENTER_CENTER,
            "Run Full Analysis • raw fundus shown",
            FontId::proportional(9.0),
            theme::TEXT_MUTED(),
        );
    }
}

/// Renders explainability attribution heatmap overlay
pub(crate) fn render_explainability_overlay(
    painter: &egui::Painter,
    display_rect: Rect,
    artifact: &crate::domain::ExplainabilityArtifact,
) {
    if let (ArtifactLoadStatus::ArtifactReady, Some(mask)) = (artifact.status, &artifact.texture) {
        // Same stored attribution as the Explainability page, drawn on the same source frame.
        painter.image(
            mask.texture.id(),
            display_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.text(
            Pos2::new(display_rect.center().x, display_rect.max.y - 10.0),
            Align2::CENTER_CENTER,
            &artifact.summary,
            FontId::proportional(9.5),
            theme::TEXT_PRIMARY(),
        );
        return;
    }
    if artifact.status != ArtifactLoadStatus::ArtifactReady {
        let badge_rect = Rect::from_center_size(display_rect.center(), Vec2::new(240.0, 48.0));
        painter.rect(
            badge_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(16, 17, 19, 210),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y - 6.0),
            Align2::CENTER_CENTER,
            "Explainability unavailable",
            FontId::proportional(11.0),
            theme::TEXT_SECONDARY(),
        );
        painter.text(
            Pos2::new(badge_rect.center().x, badge_rect.center().y + 10.0),
            Align2::CENTER_CENTER,
            "Needs a genuine Phase 4 attribution result • no heatmap is drawn",
            FontId::proportional(9.0),
            theme::TEXT_MUTED(),
        );
    }
}

// ===========================================================================
// CLINICAL REPORT & AUDIT LEDGER WORKSPACE RENDERERS (TASK 05)
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn render_clinical_report_overview(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    mode_textures: &[Option<TextureHandle>; 4],
    uploaded_filename: Option<&str>,
    uploaded_dimensions: Option<[usize; 2]>,
    stage1_result: Option<&crate::engine::Stage1ScreeningResult>,
    stage1_error: Option<&str>,
    session_export_status: Option<&str>,
    action: &mut ViewerAction,
) {
    let has_image_for_report = mode_textures[0].is_some() || exam.image_path.is_some();
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("clinical_report_scroll_area")
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            let content_w = ui.available_width().max(600.0);
            ui.set_max_width(content_w);
            ui.add_space(6.0);

            if let Some(status) = session_export_status {
                let toast_h = 32.0;
                let toast_rect = ui.allocate_space(Vec2::new(content_w, toast_h)).1;
                ui.painter().rect(
                    toast_rect,
                    CornerRadius::same(6),
                    Color32::from_rgb(16, 40, 24),
                    Stroke::new(1.0, theme::DIR_SUCCESS()),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    toast_rect.center(),
                    Align2::CENTER_CENTER,
                    format!("✓ {status}"),
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
                ui.add_space(8.0);
            }

            // 1. Clinical Header Card
            let header_h = 96.0;
            let header_rect = ui.allocate_space(Vec2::new(content_w, header_h)).1;
            ui.painter().rect(
                header_rect,
                CornerRadius::same(10),
                theme::TOP_BAR_BG(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            ui.painter().text(
                Pos2::new(header_rect.min.x + 16.0, header_rect.min.y + 20.0),
                Align2::LEFT_CENTER,
                "📋 CLINICAL SCREENING REPORT",
                FontId::proportional(15.0),
                theme::TEXT_PRIMARY(),
            );
            ui.painter().text(
                Pos2::new(header_rect.min.x + 16.0, header_rect.min.y + 40.0),
                Align2::LEFT_CENTER,
                if exam.cascade.outcome.as_ref().is_some_and(|o| o.is_ensemble()) {
                    "HierarchiRetina Stage I → II → III research cascade summary (five-fold ensemble)"
                } else {
                    "HierarchiRetina Stage I → II → III research cascade summary (technical, single fold)"
                },
                FontId::proportional(11.0),
                theme::TEXT_MUTED(),
            );

            // Metadata Chips
            let mut chip_x = header_rect.min.x + 16.0;
            let chip_y = header_rect.min.y + 70.0;
            let chips = [
                format!("PATIENT: {}", exam.patient_code),
                format!("EXAM: {}", exam.exam_id),
                format!("EYE: {:?}", exam.laterality),
            ];
            for chip in &chips {
                let w = chip.len() as f32 * 6.5 + 14.0;
                let c_rect =
                    Rect::from_min_size(Pos2::new(chip_x, chip_y - 10.0), Vec2::new(w, 20.0));
                ui.painter().rect(
                    c_rect,
                    CornerRadius::same(4),
                    theme::INNER_TILE(),
                    Stroke::new(1.0, theme::BORDER()),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    c_rect.center(),
                    Align2::CENTER_CENTER,
                    chip,
                    FontId::proportional(10.0),
                    theme::TEXT_SECONDARY(),
                );
                chip_x += w + 8.0;
            }

            // Route Status Pill on Top Right
            let (status_text, status_bg, status_color) = if let Some(res) = stage1_result {
                if res.decision == crate::engine::Stage1GateDecision::NegativeRoute {
                    (
                        "✓ NEGATIVE ROUTE (DR GRADE 0 TRIAGE)",
                        Color32::from_rgb(6, 78, 59),
                        Color32::from_rgb(16, 185, 129),
                    )
                } else {
                    (
                        "⚠ CONTINUE CASCADE (DR REFERRAL / PROGRESSION)",
                        theme::DIR_AMBER_BG(),
                        theme::DIR_AMBER(),
                    )
                }
            } else {
                (
                    "○ PENDING SCREENING EXECUTION",
                    theme::INNER_TILE(),
                    theme::TEXT_MUTED(),
                )
            };
            let status_w = 265.0;
            let status_rect = Rect::from_min_size(
                Pos2::new(
                    header_rect.max.x - status_w - 16.0,
                    header_rect.min.y + 16.0,
                ),
                Vec2::new(status_w, 24.0),
            );
            ui.painter().rect(
                status_rect,
                CornerRadius::same(5),
                status_bg,
                Stroke::new(1.0, status_color),
                StrokeKind::Inside,
            );
            ui.painter().text(
                status_rect.center(),
                Align2::CENTER_CENTER,
                status_text,
                FontId::proportional(10.0),
                Color32::WHITE,
            );

            ui.add_space(10.0);

            // 2. Fundus Scan & Stage 1 Findings
            let is_wide = content_w >= 820.0;
            if is_wide {
                let col_gap = 12.0;
                let left_w = 380.0;
                let right_w = content_w - left_w - col_gap;
                let row_h = 300.0;

                let row_rect = ui.allocate_space(Vec2::new(content_w, row_h)).1;
                let left_rect = Rect::from_min_size(row_rect.min, Vec2::new(left_w, row_h));
                let right_rect = Rect::from_min_size(
                    Pos2::new(row_rect.min.x + left_w + col_gap, row_rect.min.y),
                    Vec2::new(right_w, row_h),
                );

                render_fundus_scan_card(
                    ui,
                    left_rect,
                    exam,
                    mode_textures,
                    uploaded_filename,
                    uploaded_dimensions,
                    action,
                );
                render_stage1_findings_card(ui, right_rect, stage1_result, stage1_error, action);
            } else {
                let card_h = 280.0;
                let left_rect = ui.allocate_space(Vec2::new(content_w, card_h)).1;
                render_fundus_scan_card(
                    ui,
                    left_rect,
                    exam,
                    mode_textures,
                    uploaded_filename,
                    uploaded_dimensions,
                    action,
                );
                ui.add_space(10.0);
                let right_rect = ui.allocate_space(Vec2::new(content_w, card_h)).1;
                render_stage1_findings_card(ui, right_rect, stage1_result, stage1_error, action);
            }

            ui.add_space(10.0);

            // 3. Hierarchical Pipeline Stage Truthful Availability Matrix
            let cascade_lines =
                super::cascade_cards::report_lines(&exam.cascade, has_image_for_report);
            super::cascade_cards::card(ui, content_w, &cascade_lines);

            ui.add_space(10.0);

            // 4. Statutory Non-Diagnostic Research Caveat
            let caveat_h = 92.0;
            let caveat_rect = ui.allocate_space(Vec2::new(content_w, caveat_h)).1;
            render_statutory_caveat_card(ui, caveat_rect);

            ui.add_space(12.0);

            // 5. Actions Footer
            let footer_h = 42.0;
            let footer_rect = ui.allocate_space(Vec2::new(content_w, footer_h)).1;
            render_report_actions_footer(ui, footer_rect, exam, action);

            ui.add_space(16.0);
        });
}

fn render_fundus_scan_card(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    mode_textures: &[Option<TextureHandle>; 4],
    uploaded_filename: Option<&str>,
    uploaded_dimensions: Option<[usize; 2]>,
    _action: &mut ViewerAction,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        theme::TOP_BAR_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let head_h = 32.0;
    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 16.0),
        Align2::LEFT_CENTER,
        format!("ACQUIRED FUNDUS SCAN ({:?})", exam.laterality),
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );

    let img_container = Rect::from_min_max(
        Pos2::new(rect.min.x + 12.0, rect.min.y + head_h + 4.0),
        Pos2::new(rect.max.x - 12.0, rect.max.y - 36.0),
    );

    ui.painter().rect(
        img_container,
        CornerRadius::same(6),
        theme::INNER_TILE(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    if let Some(tex) = &mode_textures[0] {
        let tex_size = tex.size_vec2();
        let fit = aspect_fit(tex_size.x, tex_size.y, img_container.shrink(6.0));
        ui.painter().image(
            tex.id(),
            fit,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().text(
            img_container.center(),
            Align2::CENTER_CENTER,
            "No Fundus Scan Attached",
            FontId::proportional(12.0),
            theme::TEXT_MUTED(),
        );
    }

    let bottom_text = match (uploaded_filename, uploaded_dimensions) {
        (Some(name), Some(dims)) => format!("File: {} • {}x{} px", name, dims[0], dims[1]),
        _ => "Status: Awaiting input image".to_string(),
    };
    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, rect.max.y - 18.0),
        Align2::LEFT_CENTER,
        bottom_text,
        FontId::proportional(10.0),
        theme::TEXT_MUTED(),
    );
}

fn render_stage1_findings_card(
    ui: &mut Ui,
    rect: Rect,
    stage1_result: Option<&crate::engine::Stage1ScreeningResult>,
    stage1_error: Option<&str>,
    _action: &mut ViewerAction,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        theme::TOP_BAR_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 16.0),
        Align2::LEFT_CENTER,
        "STAGE I CONVNEXT-V2 SCREENING FINDINGS",
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );

    let content_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + 12.0, rect.min.y + 36.0),
        Pos2::new(rect.max.x - 12.0, rect.max.y - 12.0),
    );

    if let Some(res) = stage1_result {
        // Hero Decision Banner
        let is_negative = res.decision == crate::engine::Stage1GateDecision::NegativeRoute;
        let banner_h = 56.0;
        let banner_rect =
            Rect::from_min_size(content_rect.min, Vec2::new(content_rect.width(), banner_h));
        let (bg, border, title, desc) = if is_negative {
            (
                Color32::from_rgb(6, 78, 59),
                Color32::from_rgb(16, 185, 129),
                "✓ Route Decision: NegativeRoute (DR Grade 0 Triage)",
                "Screening score is below operational threshold (0.2391716). Low risk of referable DR.",
            )
        } else {
            (
                theme::DIR_AMBER_BG(),
                theme::DIR_AMBER(),
                "⚠ Route Decision: ContinueCascade (Referral Recommended)",
                "Screening score exceeds operational threshold (0.2391716). Specialist evaluation needed.",
            )
        };

        ui.painter().rect(
            banner_rect,
            CornerRadius::same(6),
            bg,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        ui.painter().text(
            Pos2::new(banner_rect.min.x + 12.0, banner_rect.min.y + 18.0),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(banner_rect.min.x + 12.0, banner_rect.min.y + 38.0),
            Align2::LEFT_CENTER,
            desc,
            FontId::proportional(10.0),
            Color32::from_rgb(226, 232, 240),
        );

        // Metrics Grid (Score, Threshold, Raw Logit, Latency)
        let grid_y = banner_rect.max.y + 10.0;
        let grid_h = 70.0;
        let grid_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x, grid_y),
            Vec2::new(content_rect.width(), grid_h),
        );
        ui.painter().rect(
            grid_rect,
            CornerRadius::same(6),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );

        let col_w = grid_rect.width() / 4.0;
        let metrics = [
            (
                "Screening Score",
                format!("{:.4}", res.screening_score),
                theme::ACCENT_BLUE(),
            ),
            (
                "Operational Threshold",
                format!("{:.4}", res.threshold),
                theme::TEXT_PRIMARY(),
            ),
            (
                "Raw Model Logit",
                format!("{:+.4}", res.raw_logit),
                theme::TEXT_PRIMARY(),
            ),
            (
                "Inference Latency",
                format!("{} ms", res.elapsed_ms),
                theme::TEXT_PRIMARY(),
            ),
        ];

        for (i, (lbl, val, col)) in metrics.iter().enumerate() {
            let cx = grid_rect.min.x + (i as f32 + 0.5) * col_w;
            ui.painter().text(
                Pos2::new(cx, grid_rect.min.y + 20.0),
                Align2::CENTER_CENTER,
                lbl,
                FontId::proportional(10.0),
                theme::TEXT_MUTED(),
            );
            ui.painter().text(
                Pos2::new(cx, grid_rect.min.y + 46.0),
                Align2::CENTER_CENTER,
                val,
                FontId::proportional(14.0),
                *col,
            );
        }

        // Provenance metadata card
        let prov_y = grid_rect.max.y + 8.0;
        let prov_rect = Rect::from_min_size(
            Pos2::new(content_rect.min.x, prov_y),
            Vec2::new(content_rect.width(), 44.0),
        );
        ui.painter().rect(
            prov_rect,
            CornerRadius::same(6),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            Pos2::new(prov_rect.min.x + 12.0, prov_rect.min.y + 14.0),
            Align2::LEFT_CENTER,
            format!(
                "Model Provenance: {} v{} • Provider: {}",
                res.model_id, res.model_version, res.provider
            ),
            FontId::proportional(10.0),
            theme::TEXT_SECONDARY(),
        );
        ui.painter().text(
            Pos2::new(prov_rect.min.x + 12.0, prov_rect.min.y + 30.0),
            Align2::LEFT_CENTER,
            format!("SHA-256 Checksum: {}", res.model_sha256),
            FontId::monospace(9.5),
            theme::TEXT_MUTED(),
        );
    } else if let Some(err) = stage1_error {
        ui.painter().text(
            content_rect.center(),
            Align2::CENTER_CENTER,
            format!("Screening Error: {err}"),
            FontId::proportional(12.0),
            theme::DIR_ERROR(),
        );
    } else {
        ui.painter().text(
            content_rect.center(),
            Align2::CENTER_CENTER,
            "No Stage I screening result available for this examination.",
            FontId::proportional(12.0),
            theme::TEXT_MUTED(),
        );
    }
}

/// Obsolete static availability matrix (claimed Stages II/III were not packaged). Superseded by
/// `cascade_cards::report_lines`; kept until an approved cleanup.
#[allow(dead_code)]
fn render_pipeline_matrix_card(ui: &mut Ui, rect: Rect) {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        theme::TOP_BAR_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    ui.painter().text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 16.0),
        Align2::LEFT_CENTER,
        "HIERARCHIRETINA MODULAR PIPELINE STATUS (TRUTHFUL ARCHITECTURE AUDIT)",
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );

    let rows = [
        (
            "Stage I: ConvNeXt-V2 Binary Screening",
            "● AVAILABLE",
            theme::DIR_SUCCESS(),
            Color32::from_rgb(6, 78, 59),
            "Binary first-pass classifier (NegativeRoute vs ContinueCascade). Real ONNX checkpoint verified on disk.",
        ),
        (
            "Stage II: HSMoE-AUNet Lesions & SwinHRUNet++ Vessels",
            "○ UNAVAILABLE / NOT PACKAGED",
            theme::DIR_AMBER(),
            theme::DIR_AMBER_BG(),
            "Microaneurysm, hemorrhage, exudate masks and vessel segmentation models are not packaged in this backend build.",
        ),
        (
            "Stage III: LG-DRG 5-Class Severity & Explainability",
            "○ UNAVAILABLE / NOT PACKAGED",
            theme::DIR_AMBER(),
            theme::DIR_AMBER_BG(),
            "5-Class ICDR DR grades 0-4 and Grad-CAM saliency heatmaps are not packaged. Zero clinical grades are fabricated.",
        ),
    ];

    let row_start_y = rect.min.y + 34.0;
    let row_h = 38.0;

    for (i, (stage_name, status_badge, badge_color, badge_bg, desc)) in rows.iter().enumerate() {
        let r_y = row_start_y + (i as f32) * (row_h + 4.0);
        let r_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 12.0, r_y),
            Vec2::new(rect.width() - 24.0, row_h),
        );

        ui.painter().rect(
            r_rect,
            CornerRadius::same(5),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );

        ui.painter().text(
            Pos2::new(r_rect.min.x + 10.0, r_rect.min.y + 12.0),
            Align2::LEFT_CENTER,
            *stage_name,
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );

        let badge_w = 175.0;
        let b_rect = Rect::from_min_size(
            Pos2::new(r_rect.max.x - badge_w - 10.0, r_rect.min.y + 8.0),
            Vec2::new(badge_w, 20.0),
        );
        ui.painter().rect(
            b_rect,
            CornerRadius::same(4),
            *badge_bg,
            Stroke::new(1.0, *badge_color),
            StrokeKind::Inside,
        );
        ui.painter().text(
            b_rect.center(),
            Align2::CENTER_CENTER,
            *status_badge,
            FontId::proportional(9.5),
            Color32::WHITE,
        );

        ui.painter().text(
            Pos2::new(r_rect.min.x + 10.0, r_rect.min.y + 26.0),
            Align2::LEFT_CENTER,
            *desc,
            FontId::proportional(9.5),
            theme::TEXT_MUTED(),
        );
    }
}

fn render_statutory_caveat_card(ui: &mut Ui, rect: Rect) {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        theme::DIR_AMBER_BG(),
        Stroke::new(1.2, theme::DIR_AMBER()),
        StrokeKind::Inside,
    );

    ui.painter().text(
        Pos2::new(rect.min.x + 16.0, rect.min.y + 20.0),
        Align2::LEFT_CENTER,
        "⚠️ MANDATORY RESEARCH & CLINICAL LIMITATION NOTICE",
        FontId::proportional(12.0),
        theme::DIR_AMBER(),
    );

    // Wrapped to the card width so a long notice never runs outside the frame.
    let galley = ui.painter().layout(
        crate::engine::report::MANDATORY_RESEARCH_CAVEAT.to_string(),
        FontId::proportional(10.5),
        theme::TEXT_PRIMARY(),
        (rect.width() - 32.0).max(120.0),
    );
    ui.painter().galley(
        Pos2::new(rect.min.x + 16.0, rect.min.y + 36.0),
        galley,
        theme::TEXT_PRIMARY(),
    );
}

fn render_report_actions_footer(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    action: &mut ViewerAction,
) {
    let mut cur_x = rect.min.x;
    let mid_y = rect.center().y;

    // [📄 Export PDF Report]
    let pdf_btn_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 15.0), Vec2::new(160.0, 30.0));
    cur_x += 170.0;
    let pdf_resp = ui.interact(
        pdf_btn_rect,
        Id::new("footer_pdf_export_btn"),
        Sense::click(),
    );
    if pdf_resp.hovered() {
        ui.painter()
            .rect_filled(pdf_btn_rect, CornerRadius::same(6), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        pdf_btn_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::ACCENT_BLUE()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        pdf_btn_rect.center(),
        Align2::CENTER_CENTER,
        "📄 Export PDF Report",
        FontId::proportional(11.5),
        Color32::WHITE,
    );
    if pdf_resp.clicked() {
        exam.current_subroute = ReportSubroute::PdfExport;
        *action = ViewerAction::NavigateReportSubroute(ReportSubroute::PdfExport);
    }

    // [📦 Export Patient Session]
    let sess_btn_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 15.0), Vec2::new(180.0, 30.0));
    cur_x += 190.0;
    let sess_resp = ui.interact(
        sess_btn_rect,
        Id::new("footer_sess_export_btn"),
        Sense::click(),
    );
    if sess_resp.hovered() {
        ui.painter()
            .rect_filled(sess_btn_rect, CornerRadius::same(6), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        sess_btn_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        sess_btn_rect.center(),
        Align2::CENTER_CENTER,
        "📦 Export Patient Session",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if sess_resp.clicked() {
        exam.current_subroute = ReportSubroute::SessionExport;
        *action = ViewerAction::NavigateReportSubroute(ReportSubroute::SessionExport);
    }

    // [📋 View Audit Ledger]
    let audit_btn_rect =
        Rect::from_min_size(Pos2::new(cur_x, mid_y - 15.0), Vec2::new(160.0, 30.0));
    cur_x += 170.0;
    let audit_resp = ui.interact(audit_btn_rect, Id::new("footer_audit_btn"), Sense::click());
    if audit_resp.hovered() {
        ui.painter()
            .rect_filled(audit_btn_rect, CornerRadius::same(6), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        audit_btn_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        audit_btn_rect.center(),
        Align2::CENTER_CENTER,
        "📋 View Audit Ledger",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if audit_resp.clicked() {
        exam.current_subroute = ReportSubroute::AuditHistory;
        *action = ViewerAction::NavigateReportSubroute(ReportSubroute::AuditHistory);
    }

    // [⊞ Switch to Quad View]
    let quad_btn_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 15.0), Vec2::new(160.0, 30.0));
    let quad_resp = ui.interact(quad_btn_rect, Id::new("footer_quad_btn"), Sense::click());
    if quad_resp.hovered() {
        ui.painter()
            .rect_filled(quad_btn_rect, CornerRadius::same(6), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        quad_btn_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        quad_btn_rect.center(),
        Align2::CENTER_CENTER,
        "⊞ Switch to Quad View",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    if quad_resp.clicked() {
        exam.report_view_mode = false;
        exam.current_subroute = ReportSubroute::QuadViewer;
        *action = ViewerAction::NavigateReportSubroute(ReportSubroute::QuadViewer);
    }
}

fn render_pdf_export_modal(
    ui: &mut Ui,
    screen_rect: Rect,
    exam: &mut RetinalExaminationView,
    pdf_readiness: &crate::engine::PdfServiceReadiness,
    export_state: &crate::engine::ExportState,
    action: &mut ViewerAction,
) {
    // Backdrop
    ui.painter().rect_filled(
        screen_rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );

    let modal_w = 600.0;
    let modal_h = 390.0;
    let modal_rect = Rect::from_center_size(screen_rect.center(), Vec2::new(modal_w, modal_h));

    ui.painter().rect(
        modal_rect,
        CornerRadius::same(12),
        theme::TOP_BAR_BG(),
        Stroke::new(1.5, theme::BORDER_LIGHT()),
        StrokeKind::Inside,
    );

    // Modal Title Bar
    ui.painter().text(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 24.0),
        Align2::LEFT_CENTER,
        "📄 Export Clinical Screening Summary (PDF)",
        FontId::proportional(14.0),
        theme::TEXT_PRIMARY(),
    );
    ui.painter().text(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 44.0),
        Align2::LEFT_CENTER,
        format!(
            "Examination ID: {} • Patient: {}",
            exam.exam_id, exam.patient_code
        ),
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );

    // Close Button Top-Right
    let close_rect = Rect::from_min_size(
        Pos2::new(modal_rect.max.x - 36.0, modal_rect.min.y + 16.0),
        Vec2::new(22.0, 22.0),
    );
    let close_resp = ui.interact(close_rect, Id::new("close_pdf_modal_btn"), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(4), theme::CARD_HOVER());
    }
    ui.painter().text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );
    if close_resp.clicked() {
        exam.export_dialog_open = false;
        *action = ViewerAction::CloseModals;
    }

    let body_rect = Rect::from_min_max(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 68.0),
        Pos2::new(modal_rect.max.x - 20.0, modal_rect.max.y - 60.0),
    );

    match pdf_readiness {
        crate::engine::PdfServiceReadiness::BackendUnavailable { reason } => {
            ui.painter().rect(
                body_rect,
                CornerRadius::same(8),
                theme::DIR_AMBER_BG(),
                Stroke::new(1.2, theme::DIR_AMBER()),
                StrokeKind::Inside,
            );

            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                "⚠️ PDF GENERATION ENGINE UNAVAILABLE IN BACKEND",
                FontId::proportional(12.0),
                theme::DIR_AMBER(),
            );

            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 48.0),
                Align2::LEFT_CENTER,
                format!("Backend Status: {reason}"),
                FontId::proportional(10.5),
                theme::TEXT_PRIMARY(),
            );

            let explanation = "The backend crate retinacare-reports contains only data structures (ReportData);\nit does not currently implement a PDF compilation pipeline (such as printpdf or genpdf).\nPer strict medical truthfulness, zero synthetic PDFs are claimed as genuine.";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 88.0),
                Align2::LEFT_CENTER,
                explanation,
                FontId::proportional(10.0),
                theme::TEXT_SECONDARY(),
            );

            let handoff_label = "Minimal Codex Backend API Required:";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 140.0),
                Align2::LEFT_CENTER,
                handoff_label,
                FontId::proportional(10.5),
                theme::ACCENT_BLUE(),
            );

            let api_spec = "retinacare_reports::generate_clinical_pdf(req: &ReportExportRequest) -> Result<PathBuf, ReportError>";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 162.0),
                Align2::LEFT_CENTER,
                api_spec,
                FontId::monospace(9.5),
                theme::TEXT_PRIMARY(),
            );
        }
        crate::engine::PdfServiceReadiness::Ready { provider } => {
            ui.painter().rect(
                body_rect,
                CornerRadius::same(8),
                theme::INNER_TILE(),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                format!("PDF Generation Engine: {provider}"),
                FontId::proportional(11.5),
                theme::TEXT_PRIMARY(),
            );

            match export_state {
                crate::engine::ExportState::Idle => {
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 56.0),
                        Align2::LEFT_CENTER,
                        "Ready to compile clinical report. Target: /tmp/retinacare_report.pdf",
                        FontId::proportional(11.0),
                        theme::TEXT_SECONDARY(),
                    );
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 80.0),
                        Align2::LEFT_CENTER,
                        "Includes: Examination metadata, Fundus scan, Stage 1 findings, Statutory Caveat",
                        FontId::proportional(10.0),
                        theme::TEXT_MUTED(),
                    );
                }
                crate::engine::ExportState::Generating { request_id } => {
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 56.0),
                        Align2::LEFT_CENTER,
                        format!("Rendering PDF document... (Request ID: {request_id})"),
                        FontId::proportional(11.0),
                        theme::ACCENT_BLUE(),
                    );
                }
                crate::engine::ExportState::Saved { path, bytes, .. } => {
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 56.0),
                        Align2::LEFT_CENTER,
                        format!("✓ PDF Successfully Generated and Saved! ({} bytes)", bytes),
                        FontId::proportional(11.5),
                        theme::DIR_SUCCESS(),
                    );
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 80.0),
                        Align2::LEFT_CENTER,
                        format!("Destination: {}", path.display()),
                        FontId::monospace(10.0),
                        theme::TEXT_PRIMARY(),
                    );
                }
                crate::engine::ExportState::Failed { error, .. } => {
                    ui.painter().text(
                        Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 56.0),
                        Align2::LEFT_CENTER,
                        format!("✕ PDF Export Failed: {error}"),
                        FontId::proportional(11.5),
                        theme::DIR_ERROR(),
                    );
                }
                _ => {}
            }
        }
    }

    // Bottom Action Row
    let action_y = modal_rect.max.y - 36.0;
    let cancel_btn_rect = Rect::from_min_size(
        Pos2::new(modal_rect.max.x - 90.0, action_y - 14.0),
        Vec2::new(70.0, 28.0),
    );
    let cancel_resp = ui.interact(
        cancel_btn_rect,
        Id::new("close_pdf_btn_bottom"),
        Sense::click(),
    );
    if cancel_resp.hovered() {
        ui.painter()
            .rect_filled(cancel_btn_rect, CornerRadius::same(5), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        cancel_btn_rect,
        CornerRadius::same(5),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        cancel_btn_rect.center(),
        Align2::CENTER_CENTER,
        "Close",
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
    if cancel_resp.clicked() {
        exam.export_dialog_open = false;
        *action = ViewerAction::CloseModals;
    }

    if let crate::engine::PdfServiceReadiness::Ready { .. } = pdf_readiness {
        let gen_btn_rect = Rect::from_min_size(
            Pos2::new(cancel_btn_rect.min.x - 140.0, action_y - 14.0),
            Vec2::new(130.0, 28.0),
        );
        let gen_resp = ui.interact(
            gen_btn_rect,
            Id::new("submit_pdf_export_btn"),
            Sense::click(),
        );
        if gen_resp.hovered() {
            ui.painter()
                .rect_filled(gen_btn_rect, CornerRadius::same(5), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            gen_btn_rect,
            CornerRadius::same(5),
            Stroke::new(1.0, theme::ACCENT_BLUE()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            gen_btn_rect.center(),
            Align2::CENTER_CENTER,
            "💾 Generate PDF",
            FontId::proportional(11.0),
            Color32::WHITE,
        );
        if gen_resp.clicked() {
            *action = ViewerAction::SubmitPdfExport;
        }
    }
}

fn render_audit_ledger_modal(
    ui: &mut Ui,
    screen_rect: Rect,
    exam: &mut RetinalExaminationView,
    audit_readiness: &crate::engine::AuditLedgerReadiness,
    audit_events: &[crate::engine::AuditEvent],
    action: &mut ViewerAction,
) {
    // Backdrop
    ui.painter().rect_filled(
        screen_rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );

    let modal_w = 780.0;
    let modal_h = 480.0;
    let modal_rect = Rect::from_center_size(screen_rect.center(), Vec2::new(modal_w, modal_h));

    ui.painter().rect(
        modal_rect,
        CornerRadius::same(12),
        theme::TOP_BAR_BG(),
        Stroke::new(1.5, theme::BORDER_LIGHT()),
        StrokeKind::Inside,
    );

    // Header
    ui.painter().text(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 24.0),
        Align2::LEFT_CENTER,
        "📋 Append-Only Examination Audit Ledger",
        FontId::proportional(14.0),
        theme::TEXT_PRIMARY(),
    );
    ui.painter().text(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 44.0),
        Align2::LEFT_CENTER,
        format!(
            "Filtered to Examination: {} (Patient: {})",
            exam.exam_id, exam.patient_code
        ),
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );

    let close_rect = Rect::from_min_size(
        Pos2::new(modal_rect.max.x - 36.0, modal_rect.min.y + 16.0),
        Vec2::new(22.0, 22.0),
    );
    let close_resp = ui.interact(close_rect, Id::new("close_audit_modal_btn"), Sense::click());
    if close_resp.hovered() {
        ui.painter()
            .rect_filled(close_rect, CornerRadius::same(4), theme::CARD_HOVER());
    }
    ui.painter().text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );
    if close_resp.clicked() {
        exam.audit_ledger_open = false;
        *action = ViewerAction::CloseModals;
    }

    let body_rect = Rect::from_min_max(
        Pos2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 68.0),
        Pos2::new(modal_rect.max.x - 20.0, modal_rect.max.y - 50.0),
    );

    match audit_readiness {
        crate::engine::AuditLedgerReadiness::Unavailable { reason } => {
            ui.painter().rect(
                body_rect,
                CornerRadius::same(8),
                theme::DIR_AMBER_BG(),
                Stroke::new(1.2, theme::DIR_AMBER()),
                StrokeKind::Inside,
            );

            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 24.0),
                Align2::LEFT_CENTER,
                "ℹ️ AUDIT HISTORY UNAVAILABLE IN PERSISTENCE STORE",
                FontId::proportional(12.0),
                theme::DIR_AMBER(),
            );

            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 48.0),
                Align2::LEFT_CENTER,
                format!("Persistence Status: {reason}"),
                FontId::proportional(10.5),
                theme::TEXT_PRIMARY(),
            );

            let explanation = "The backend persistence store (retinacare-persistence SqlCipherStore) manages patient\nand examination records, but does not currently include an audit_log table or event ledger query API.\nPer zero-hallucination policy, no synthetic audit entries are fabricated.";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 88.0),
                Align2::LEFT_CENTER,
                explanation,
                FontId::proportional(10.0),
                theme::TEXT_SECONDARY(),
            );

            let handoff_label = "Minimal Codex Backend Persistence API Required:";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 140.0),
                Align2::LEFT_CENTER,
                handoff_label,
                FontId::proportional(10.5),
                theme::ACCENT_BLUE(),
            );

            let api_spec = "retinacare_persistence::record_audit_event(event: &AuditEvent) -> Result<(), PersistenceError>\nretinacare_persistence::query_audit_events(filter: &AuditFilter) -> Result<Vec<AuditEvent>, PersistenceError>";
            ui.painter().text(
                Pos2::new(body_rect.min.x + 16.0, body_rect.min.y + 164.0),
                Align2::LEFT_CENTER,
                api_spec,
                FontId::monospace(9.5),
                theme::TEXT_PRIMARY(),
            );
        }
        crate::engine::AuditLedgerReadiness::Ready { .. } => {
            let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
            egui::ScrollArea::vertical()
                .id_salt("audit_events_scroll_area")
                .show(&mut list_ui, |ui| {
                    let table_w = ui.available_width();
                    let row_h = 36.0;

                    if audit_events.is_empty() {
                        ui.painter().text(
                            body_rect.center(),
                            Align2::CENTER_CENTER,
                            "No audit records recorded for this examination session.",
                            FontId::proportional(11.0),
                            theme::TEXT_MUTED(),
                        );
                    } else {
                        for ev in audit_events {
                            let (row_rect, _) =
                                ui.allocate_exact_size(Vec2::new(table_w, row_h), Sense::hover());
                            ui.painter().rect(
                                row_rect,
                                CornerRadius::same(4),
                                theme::INNER_TILE(),
                                Stroke::new(1.0, theme::BORDER()),
                                StrokeKind::Inside,
                            );

                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 8.0, row_rect.center().y),
                                Align2::LEFT_CENTER,
                                &ev.timestamp,
                                FontId::monospace(9.5),
                                theme::TEXT_MUTED(),
                            );

                            let type_str = format!("{:?}", ev.event_type);
                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 175.0, row_rect.center().y),
                                Align2::LEFT_CENTER,
                                type_str,
                                FontId::proportional(10.0),
                                theme::TEXT_PRIMARY(),
                            );

                            let (status_str, status_col) = match ev.status {
                                crate::engine::AuditEventStatus::Success => {
                                    ("SUCCESS", theme::DIR_SUCCESS())
                                }
                                crate::engine::AuditEventStatus::Failed => {
                                    ("FAILED", theme::DIR_ERROR())
                                }
                                crate::engine::AuditEventStatus::Cancelled => {
                                    ("CANCELLED", theme::DIR_AMBER())
                                }
                                crate::engine::AuditEventStatus::Informational => {
                                    ("INFO", theme::ACCENT_BLUE())
                                }
                            };

                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 335.0, row_rect.center().y),
                                Align2::LEFT_CENTER,
                                status_str,
                                FontId::proportional(9.5),
                                status_col,
                            );

                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 405.0, row_rect.center().y),
                                Align2::LEFT_CENTER,
                                &ev.non_sensitive_summary,
                                FontId::proportional(10.0),
                                theme::TEXT_SECONDARY(),
                            );
                        }
                    }
                });
        }
    }

    // Close Button at Bottom
    let bottom_close_rect = Rect::from_min_size(
        Pos2::new(modal_rect.max.x - 90.0, modal_rect.max.y - 38.0),
        Vec2::new(70.0, 26.0),
    );
    let bc_resp = ui.interact(
        bottom_close_rect,
        Id::new("close_audit_btn_bottom"),
        Sense::click(),
    );
    if bc_resp.hovered() {
        ui.painter().rect_filled(
            bottom_close_rect,
            CornerRadius::same(5),
            theme::CARD_HOVER(),
        );
    }
    ui.painter().rect_stroke(
        bottom_close_rect,
        CornerRadius::same(5),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        bottom_close_rect.center(),
        Align2::CENTER_CENTER,
        "Close",
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
    if bc_resp.clicked() {
        exam.audit_ledger_open = false;
        *action = ViewerAction::CloseModals;
    }
}

fn render_report_subroute_toolbar(
    ui: &mut Ui,
    rect: Rect,
    exam: &RetinalExaminationView,
    subroute_title: &str,
    action: &mut ViewerAction,
) {
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        theme::TOP_BAR_BG(),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let mid_y = rect.center().y;
    let mut cur_x = rect.min.x + 12.0;

    // Back to Report Workspace Button
    let back_w = 130.0;
    let back_rect = Rect::from_min_size(Pos2::new(cur_x, mid_y - 13.0), Vec2::new(back_w, 26.0));
    cur_x += back_w + 14.0;

    let back_resp = ui.interact(
        back_rect,
        Id::new("report_subroute_back_btn"),
        Sense::click(),
    );
    if back_resp.hovered() {
        ui.painter()
            .rect_filled(back_rect, CornerRadius::same(6), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        back_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        back_rect.center(),
        Align2::CENTER_CENTER,
        "← Back to Report",
        FontId::proportional(11.0),
        theme::TEXT_PRIMARY(),
    );
    if back_resp.clicked() {
        *action = ViewerAction::BackToReportWorkspace;
    }

    // Breadcrumb title
    let title_str = format!("Report  /  {} ({})", subroute_title, exam.exam_id);
    ui.painter().text(
        Pos2::new(cur_x, mid_y),
        Align2::LEFT_CENTER,
        title_str,
        FontId::proportional(12.0),
        theme::TEXT_PRIMARY(),
    );

    // Right-aligned chips: Patient & Eye laterality
    let lat_str = match exam.laterality {
        EyeLaterality::OD => "OD (Right Eye)",
        EyeLaterality::OS => "OS (Left Eye)",
        EyeLaterality::Unspecified => "Unspecified Laterality",
    };
    let badge_text = format!("Patient: {}  •  {}", exam.patient_code, lat_str);
    ui.painter().text(
        Pos2::new(rect.max.x - 14.0, mid_y),
        Align2::RIGHT_CENTER,
        badge_text,
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );
}

fn render_pdf_export_screen(
    ui: &mut Ui,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    pdf_readiness: &crate::engine::PdfServiceReadiness,
    export_state: &crate::engine::ExportState,
    action: &mut ViewerAction,
) {
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("pdf_export_screen_scroll")
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("📄 Clinical Screening Summary Export (PDF)")
                    .font(FontId::proportional(16.0))
                    .color(theme::TEXT_PRIMARY())
                    .strong(),
            );
            ui.label(
                egui::RichText::new(
                    "Generate a research report (PDF; text, plus a picture grid of the stored masks and the AI Explainability overlay only when a saved, verified result is open) from this examination: identification, status, the verified Stage I / II / III results that exist, provenance and limitations."
                )
                .font(FontId::proportional(11.5))
                .color(theme::TEXT_MUTED()),
            );

            ui.add_space(14.0);

            // Target Examination Context Card
            egui::Frame::new()
                .fill(theme::CARD_BG())
                .stroke(Stroke::new(1.0, theme::BORDER()))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("TARGET CLINICAL CONTEXT")
                            .font(FontId::proportional(11.0))
                            .color(theme::ACCENT_BLUE())
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Patient Code: {}", exam.patient_code))
                                .font(FontId::proportional(12.0))
                                .color(theme::TEXT_PRIMARY()),
                        );
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(format!("Examination ID: {}", exam.exam_id))
                                .font(FontId::proportional(12.0))
                                .color(theme::TEXT_PRIMARY()),
                        );
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(format!("Laterality: {:?}", exam.laterality))
                                .font(FontId::proportional(12.0))
                                .color(theme::TEXT_SECONDARY()),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Destination: you choose the file when you export (a save dialog opens).")
                            .font(FontId::monospace(10.5))
                            .color(theme::TEXT_MUTED()),
                    );
                });

            ui.add_space(14.0);

            // PDF Service Readiness Banner
            match pdf_readiness {
                crate::engine::PdfServiceReadiness::BackendUnavailable { reason } => {
                    egui::Frame::new()
                        .fill(theme::DIR_AMBER_BG())
                        .stroke(Stroke::new(1.2, theme::DIR_AMBER()))
                        .corner_radius(8.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("⚠️ PDF GENERATION ENGINE UNAVAILABLE IN BACKEND")
                                    .font(FontId::proportional(13.0))
                                    .color(theme::DIR_AMBER())
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(format!("Backend Status: {reason}"))
                                    .font(FontId::proportional(11.0))
                                    .color(theme::TEXT_PRIMARY()),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(
                                    "The backend crate retinacare-reports contains data schemas (ReportData) but does not implement a PDF compilation pipeline (such as printpdf or genpdf).\nIn compliance with clinical truthfulness and zero-hallucination policy, no synthetic or empty PDF file will be created."
                                )
                                .font(FontId::proportional(10.5))
                                .color(theme::TEXT_SECONDARY()),
                            );
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("Minimal Required Codex Backend API:")
                                    .font(FontId::proportional(11.0))
                                    .color(theme::ACCENT_BLUE())
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new("retinacare_reports::generate_clinical_pdf(req: &ReportExportRequest) -> Result<PathBuf, ReportError>")
                                    .font(FontId::monospace(10.0))
                                    .color(theme::TEXT_PRIMARY()),
                            );
                        });

                    ui.add_space(14.0);

                    // Disabled Generation Button
                    ui.add_enabled_ui(false, |ui| {
                        let _ = ui.button(
                            egui::RichText::new("💾 Generate PDF (Unavailable in Backend)")
                                .font(FontId::proportional(12.0)),
                        ).on_disabled_hover_text("PDF generation engine has not been implemented in retinacare-reports backend crate.");
                    });
                }
                crate::engine::PdfServiceReadiness::Ready { provider } => {
                    egui::Frame::new()
                        .fill(theme::INNER_TILE())
                        .stroke(Stroke::new(1.0, theme::BORDER()))
                        .corner_radius(8.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("✓ PDF Engine Ready: {provider}"))
                                    .font(FontId::proportional(12.5))
                                    .color(theme::DIR_SUCCESS())
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            match export_state {
                                crate::engine::ExportState::Idle => {
                                    ui.label(
                                        egui::RichText::new("Ready to write the PDF atomically: a failed write never leaves a partial file.")
                                            .font(FontId::proportional(11.0))
                                            .color(theme::TEXT_SECONDARY()),
                                    );
                                }
                                crate::engine::ExportState::Generating { request_id } => {
                                    ui.label(
                                        egui::RichText::new(format!("Rendering PDF document... (Request ID: {request_id})"))
                                            .font(FontId::proportional(11.0))
                                            .color(theme::ACCENT_BLUE()),
                                    );
                                }
                                crate::engine::ExportState::Saved { path, bytes, .. } => {
                                    ui.label(
                                        egui::RichText::new(format!("✓ Successfully generated {} bytes to {}", bytes, path.display()))
                                            .font(FontId::proportional(11.5))
                                            .color(theme::DIR_SUCCESS()),
                                    );
                                }
                                crate::engine::ExportState::Failed { error, .. } => {
                                    ui.label(
                                        egui::RichText::new(format!("✕ Export Failed: {error}"))
                                            .font(FontId::proportional(11.5))
                                            .color(theme::DIR_ERROR()),
                                    );
                                }
                                _ => {}
                            }
                        });

                    ui.add_space(14.0);

                    if ui.button(
                        egui::RichText::new("💾 Compile & Export Clinical PDF")
                            .font(FontId::proportional(12.0))
                            .color(Color32::WHITE),
                    ).clicked() {
                        *action = ViewerAction::SubmitPdfExport;
                    }
                }
            }

            ui.add_space(20.0);

            // Scope & Audit Details
            egui::Frame::new()
                .fill(theme::CARD_BG())
                .stroke(Stroke::new(1.0, theme::BORDER()))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("INCLUDED CLINICAL DOCUMENTATION SPECIFICATION")
                            .font(FontId::proportional(10.5))
                            .color(theme::TEXT_MUTED())
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("• 1. Identification: patient code, examination, eye, source image, generation time").font(FontId::proportional(10.5)).color(theme::TEXT_SECONDARY()));
                    ui.label(egui::RichText::new("• 2. Status: complete, partial or no result - never implied").font(FontId::proportional(10.5)).color(theme::TEXT_SECONDARY()));
                    ui.label(egui::RichText::new("• 3. Results: only stages that actually ran (no grade for failed or cancelled runs)").font(FontId::proportional(10.5)).color(theme::TEXT_SECONDARY()));
                    ui.label(egui::RichText::new("• 4. Provenance: model hashes, providers, image hash, identity checks, timings").font(FontId::proportional(10.5)).color(theme::TEXT_SECONDARY()));
                    ui.label(egui::RichText::new("• 5. Limitations: research-only, model scope as stated, CPU only; 6. Segmentation evidence picture (only for a saved verified run); 7. AI Explainability (only if a verified result is open)").font(FontId::proportional(10.5)).color(theme::TEXT_SECONDARY()));
                });
        });
}

fn render_session_export_screen(
    ui: &mut Ui,
    rect: Rect,
    exam: &RetinalExaminationView,
    session_export_status: Option<&str>,
    action: &mut ViewerAction,
) {
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("session_export_screen_scroll")
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("📦 Scoped Patient Session Dossier Export")
                    .font(FontId::proportional(16.0))
                    .color(theme::TEXT_PRIMARY())
                    .strong(),
            );
            ui.label(
                egui::RichText::new(
                    "Export complete clinical examination session data, registered imaging metadata, and model inferences to a standalone JSON dossier."
                )
                .font(FontId::proportional(11.5))
                .color(theme::TEXT_MUTED()),
            );

            ui.add_space(14.0);

            // Warning Banner: Plaintext Sensitive Export
            egui::Frame::new()
                .fill(theme::DIR_AMBER_BG())
                .stroke(Stroke::new(1.2, theme::DIR_AMBER()))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("⚠️ PLAINTEXT SENSITIVE EXPORT (UNENCRYPTED JSON)")
                            .font(FontId::proportional(13.0))
                            .color(theme::DIR_AMBER())
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "This operation outputs unencrypted patient records and examination findings directly to the local filesystem.\nDo not save to unsecured public directories or unencrypted removable media."
                        )
                        .font(FontId::proportional(11.0))
                        .color(theme::TEXT_PRIMARY()),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Strict Scope Guard: This export is strictly isolated to Patient '{}' and Examination '{}'. Prohibits cross-patient leakage.",
                            exam.patient_code, exam.exam_id
                        ))
                        .font(FontId::proportional(10.5))
                        .color(theme::ACCENT_BLUE()),
                    );
                });

            ui.add_space(14.0);

            // Target Destination & Feedback
            egui::Frame::new()
                .fill(theme::CARD_BG())
                .stroke(Stroke::new(1.0, theme::BORDER()))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("EXPORT CONFIGURATION")
                            .font(FontId::proportional(11.0))
                            .color(theme::TEXT_MUTED())
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Target Destination: /tmp/retinacare_patient_{}_session.json",
                            exam.patient_code
                        ))
                        .font(FontId::monospace(11.0))
                        .color(theme::TEXT_PRIMARY()),
                    );

                    if let Some(status) = session_export_status {
                        ui.add_space(8.0);
                        let is_err = status.contains("failed") || status.contains("error");
                        ui.label(
                            egui::RichText::new(status)
                                .font(FontId::proportional(11.0))
                                .color(if is_err {
                                    theme::DIR_ERROR()
                                } else {
                                    theme::DIR_SUCCESS()
                                }),
                        );
                    }
                });

            ui.add_space(14.0);

            // Primary Export Button
            if ui.button(
                egui::RichText::new("📦 Export Scoped Session Dossier (JSON)")
                    .font(FontId::proportional(12.0))
                    .color(Color32::WHITE),
            ).clicked() {
                *action = ViewerAction::SubmitSessionExport;
            }
        });
}

fn render_audit_history_screen(
    ui: &mut Ui,
    rect: Rect,
    exam: &RetinalExaminationView,
    audit_readiness: &crate::engine::AuditLedgerReadiness,
    audit_events: &[crate::engine::AuditEvent],
    _action: &mut ViewerAction,
) {
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("audit_history_screen_scroll")
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("📋 Append-Only Examination Audit Ledger")
                    .font(FontId::proportional(16.0))
                    .color(theme::TEXT_PRIMARY())
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Recorded event history (append-only table, not cryptographically chained) filtered to Examination ID: {} (Patient: {})",
                    exam.exam_id, exam.patient_code
                ))
                .font(FontId::proportional(11.5))
                .color(theme::TEXT_MUTED()),
            );

            ui.add_space(14.0);

            match audit_readiness {
                crate::engine::AuditLedgerReadiness::Unavailable { reason } => {
                    egui::Frame::new()
                            .fill(theme::DIR_AMBER_BG())
                            .stroke(Stroke::new(1.2, theme::DIR_AMBER()))
                            .corner_radius(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("ℹ️ AUDIT HISTORY UNAVAILABLE IN PERSISTENCE STORE")
                                        .font(FontId::proportional(13.0))
                                        .color(theme::DIR_AMBER())
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(format!("Persistence Status: {reason}"))
                                        .font(FontId::proportional(11.0))
                                        .color(theme::TEXT_PRIMARY()),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(
                                        "The backend persistence store (retinacare-persistence SqlCipherStore) manages patient and examination records, but does not currently include an audit_log table or event ledger query API.\nPer zero-hallucination policy, no synthetic audit entries are fabricated."
                                    )
                                    .font(FontId::proportional(10.5))
                                    .color(theme::TEXT_SECONDARY()),
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new("Minimal Required Codex Backend Persistence API:")
                                        .font(FontId::proportional(11.0))
                                        .color(theme::ACCENT_BLUE())
                                        .strong(),
                                );
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new("retinacare_persistence::record_audit_event(event: &AuditEvent) -> Result<(), PersistenceError>\nretinacare_persistence::query_audit_events(filter: &AuditFilter) -> Result<Vec<AuditEvent>, PersistenceError>")
                                        .font(FontId::monospace(10.0))
                                        .color(theme::TEXT_PRIMARY()),
                                );
                            });
                    }
                    crate::engine::AuditLedgerReadiness::Ready { record_count } => {
                        egui::Frame::new()
                            .fill(theme::INNER_TILE())
                            .stroke(Stroke::new(1.0, theme::BORDER()))
                            .corner_radius(8.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("✓ Audit Store Ready ({record_count} records)"))
                                        .font(FontId::proportional(11.5))
                                        .color(theme::DIR_SUCCESS())
                                        .strong(),
                                );
                            });
                    }
                }

                ui.add_space(14.0);

                // Audit Events Table or Empty State
                if audit_events.is_empty() {
                    egui::Frame::new()
                        .fill(theme::CARD_BG())
                        .stroke(Stroke::new(1.0, theme::BORDER()))
                        .corner_radius(8.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "No audit entries recorded for Examination '{}'.",
                                    exam.exam_id
                                ))
                                .font(FontId::proportional(11.5))
                                .color(theme::TEXT_MUTED()),
                            );
                        });
                } else {
                    egui::Frame::new()
                        .fill(theme::CARD_BG())
                        .stroke(Stroke::new(1.0, theme::BORDER()))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("VERIFIED AUDIT LOG EVENTS")
                                        .font(FontId::proportional(11.0))
                                        .color(theme::ACCENT_BLUE())
                                        .strong(),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(format!("({} events)", audit_events.len()))
                                        .font(FontId::proportional(10.5))
                                        .color(theme::TEXT_MUTED()),
                                );
                            });
                            ui.add_space(8.0);

                            for event in audit_events {
                                egui::Frame::new()
                                    .fill(theme::INNER_TILE())
                                    .stroke(Stroke::new(1.0, theme::BORDER()))
                                    .corner_radius(6.0)
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&event.timestamp)
                                                    .font(FontId::monospace(10.0))
                                                    .color(theme::TEXT_MUTED()),
                                            );
                                            ui.add_space(10.0);
                                            ui.label(
                                                egui::RichText::new(format!("{:?}", event.event_type))
                                                    .font(FontId::proportional(11.0))
                                                    .color(theme::TEXT_PRIMARY())
                                                    .strong(),
                                            );
                                            ui.add_space(10.0);
                                            let st_color = match event.status {
                                                crate::engine::AuditEventStatus::Success => theme::DIR_SUCCESS(),
                                                crate::engine::AuditEventStatus::Failed => theme::DIR_ERROR(),
                                                _ => theme::ACCENT_BLUE(),
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{:?}", event.status))
                                                    .font(FontId::proportional(10.0))
                                                    .color(st_color),
                                            );
                                        });
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(&event.non_sensitive_summary)
                                                .font(FontId::proportional(10.5))
                                                .color(theme::TEXT_SECONDARY()),
                                        );
                                    });
                                ui.add_space(6.0);
                            }
                        });
                }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_fit_wide_vs_tall() {
        let container = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0));

        // 2:1 wide image in 400x300 container -> width = 400, height = 200
        let fit_wide = aspect_fit(800.0, 400.0, container);
        assert!((fit_wide.width() - 400.0).abs() < 0.01);
        assert!((fit_wide.height() - 200.0).abs() < 0.01);
        assert_eq!(fit_wide.center(), container.center());

        // 1:2 tall image in 400x300 container -> height = 300, width = 150
        let fit_tall = aspect_fit(400.0, 800.0, container);
        assert!((fit_tall.width() - 150.0).abs() < 0.01);
        assert!((fit_tall.height() - 300.0).abs() < 0.01);
        assert_eq!(fit_tall.center(), container.center());
    }

    #[test]
    fn test_compute_display_rect_scale() {
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(500.0, 500.0));
        let (fit_base, display_rect) =
            compute_display_rect(viewport, 1000.0, 1000.0, 2.0, Vec2::new(10.0, -10.0));

        assert!((display_rect.width() - fit_base.width() * 2.0).abs() < 0.01);
        assert_eq!(
            display_rect.center(),
            fit_base.center() + Vec2::new(10.0, -10.0)
        );
    }

    #[test]
    fn test_screen_to_image_coords_mapping() {
        let display = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 400.0));
        let img_w = 1000.0;
        let img_h = 1000.0;

        // Inside display rect (exact center)
        let center = Pos2::new(300.0, 300.0);
        let res = screen_to_image_coords(center, display, img_w, img_h);
        assert!(res.is_some());
        let (px, norm) = res.unwrap();
        assert_eq!(px, (500, 500));
        assert!((norm.0 - 0.5).abs() < 0.001);
        assert!((norm.1 - 0.5).abs() < 0.001);

        // Outside display rect (letterbox margin)
        let outside = Pos2::new(50.0, 50.0);
        assert!(screen_to_image_coords(outside, display, img_w, img_h).is_none());
    }

    #[test]
    fn test_cursor_centered_zoom() {
        let fit_base = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0));
        let mouse_pos = Pos2::new(200.0, 200.0); // Exact center

        let (new_zoom, new_pan) =
            cursor_centered_zoom(mouse_pos, 1.5, 1.0, Vec2::ZERO, fit_base, 0.5, 5.0);
        assert_eq!(new_zoom, 1.5);
        // Center zoom with cursor at center yields zero pan offset
        assert!((new_pan.x).abs() < 0.01);
        assert!((new_pan.y).abs() < 0.01);
    }
}

#[cfg(test)]
mod pan_clamp_tests {
    use super::*;

    #[test]
    fn image_cannot_be_dragged_out_of_its_panel() {
        let view = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::splat(420.0));
        let (fit, shown) = compute_display_rect(view, 960.0, 720.0, 1.0, Vec2::new(300.0, -80.0));
        assert_eq!(shown.center(), fit.center(), "zoom 1 allows no pan");
        let (_, zoomed) = compute_display_rect(view, 960.0, 720.0, 2.0, Vec2::new(900.0, 0.0));
        assert!(zoomed.min.x <= view.min.x + 0.01 && zoomed.max.x >= view.max.x - 0.01);
    }
}
