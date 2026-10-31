//! App glue for the optional cloud account: worker events, the status strip on Previous Chats and the
//! sign-in window. The password lives only in the text field and is wiped as soon as it is submitted.

use crate::engine::cloud::{CloudCommand, CloudEvent, CloudState};
use crate::ui::theme;
use crate::RetinaCareApp;
use eframe::egui::{self, Rect, RichText, Vec2};
use zeroize::Zeroize;

pub const STRIP_HEIGHT: f32 = 40.0;

impl RetinaCareApp {
    /// Drain worker events (called every frame).
    pub(crate) fn cloud_poll(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        if let Some(cloud) = &self.cloud {
            while let Ok(event) = cloud.events.try_recv() {
                events.push(event);
                if events.len() >= 64 {
                    break;
                }
            }
        }
        for event in events {
            match event {
                CloudEvent::State(state) => {
                    if matches!(state, CloudState::Connected { .. }) {
                        self.cloud_window_open = false;
                    }
                    self.cloud_state = state;
                }
                CloudEvent::Note(note) => self.cloud_note = Some(note),
                CloudEvent::LocalChanged => self.reload_sessions(),
            }
            ctx.request_repaint();
        }
    }

    pub(crate) fn cloud_signed_in(&self) -> bool {
        self.cloud_state.signed_in()
    }

    /// Status strip at the top of Previous Chats; the sign-in window floats above everything.
    pub(crate) fn render_cloud_strip(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        ctx: &egui::Context,
    ) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Cloud sync (optional)")
                    .size(11.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY()),
            );
            ui.label(
                RichText::new(self.cloud_state.label())
                    .size(11.5)
                    .color(theme::TEXT_SECONDARY()),
            );
            if self.cloud.is_none() {
                ui.label(
                    RichText::new("· not available in this build")
                        .size(11.0)
                        .color(theme::TEXT_MUTED()),
                );
                return;
            }
            if self.cloud_signed_in() {
                if ui.small_button("Sync now").clicked() {
                    self.cloud_send(CloudCommand::SyncNow);
                }
                if ui.small_button("Sign out").clicked() {
                    self.cloud_send(CloudCommand::SignOut);
                }
            } else if ui.small_button("Sign in...").clicked() {
                self.cloud_window_open = true;
            }
            if let Some(note) = &self.cloud_note {
                ui.label(RichText::new(note).size(11.0).color(theme::DIR_AMBER()));
            }
        });
        self.render_cloud_window(ctx);
    }

    fn cloud_send(&self, command: CloudCommand) {
        if let Some(cloud) = &self.cloud {
            cloud.send(command);
        }
    }

    fn render_cloud_window(&mut self, ctx: &egui::Context) {
        if !self.cloud_window_open {
            return;
        }
        let locked = !self.persistence_available();
        let mut open = true;
        egui::Window::new("Cloud account")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.set_max_width(340.0);
                ui.label(
                    RichText::new("Optional. Sign in with your own account to keep general chats in a private cloud space only you can read.")
                        .size(11.5)
                        .color(theme::TEXT_SECONDARY()),
                );
                ui.add_space(4.0);
                for line in [
                    "Patient records, retinal images, reports and analysis results are never uploaded.",
                    "Only chats you mark for sync are sent, text only.",
                    "Your password is never stored; the session is kept in the OS keyring.",
                ] {
                    ui.label(RichText::new(format!("• {line}")).size(11.0).color(theme::TEXT_MUTED()));
                }
                ui.add_space(8.0);
                if locked {
                    ui.label(
                        RichText::new("Unlock the encrypted database (Patients page) first: sync reads and writes it.")
                            .size(11.5)
                            .color(theme::DIR_AMBER()),
                    );
                    return;
                }
                ui.label(RichText::new("Email").size(11.0).color(theme::TEXT_MUTED()));
                ui.add(egui::TextEdit::singleline(&mut self.cloud_email).desired_width(320.0));
                ui.label(RichText::new("Password").size(11.0).color(theme::TEXT_MUTED()));
                let password = ui.add(
                    egui::TextEdit::singleline(&mut self.cloud_password)
                        .password(true)
                        .desired_width(320.0),
                );
                let enter = password.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(6.0);
                let ready = !self.cloud_email.trim().is_empty() && !self.cloud_password.is_empty();
                let busy = self.cloud_state == CloudState::Syncing;
                if ui.add_enabled(ready && !busy, egui::Button::new("Sign in")).clicked() || (enter && ready && !busy) {
                    let password = zeroize::Zeroizing::new(std::mem::take(&mut self.cloud_password));
                    self.cloud_password.zeroize();
                    self.cloud_note = None;
                    self.cloud_send(CloudCommand::SignIn { email: self.cloud_email.clone(), password });
                }
                if let Some(note) = &self.cloud_note {
                    ui.add_space(4.0);
                    ui.label(RichText::new(note).size(11.0).color(theme::DIR_AMBER()));
                }
                ui.add_space(2.0);
                ui.allocate_space(Vec2::new(0.0, 2.0));
            });
        if !open {
            self.cloud_window_open = false;
            self.cloud_password.zeroize();
        }
    }
}
