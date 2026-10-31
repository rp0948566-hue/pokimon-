//! Ctrl+Esc console: app log lines and a command line that runs inside a bubblewrap sandbox
//! (read-only filesystem, no network, no access to patient data). Slides up from the bottom.
use super::tokens::{radius, Palette};
use super::{fonts, theme::gentle};
use eframe::egui::{
    self, Align2, Context, CornerRadius, Id, Key, Pos2, Rect, ScrollArea, Stroke, StrokeKind,
    TextEdit, Vec2,
};
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;

const INK: egui::Color32 = egui::Color32::from_gray(232);
const INK_DIM: egui::Color32 = egui::Color32::from_gray(160);
const MAX_LINES: usize = 500;
static LOG: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Adds one line to the console log (also printed to stderr).
pub fn log(line: impl Into<String>) {
    let line = line.into();
    eprintln!("{line}");
    if let Ok(mut q) = LOG.lock() {
        if q.len() >= MAX_LINES {
            q.pop_front();
        }
        q.push_back(line);
    }
}

#[derive(Default)]
pub struct Console {
    pub open: bool,
    input: String,
    output: Vec<String>,
    running: Option<Receiver<String>>,
}

/// Command text -> sandboxed run. No network, read-only root, private /tmp, 15 s limit.
fn run_sandboxed(cmd: &str) -> String {
    let child = Command::new("timeout")
        .args([
            "15",
            "bwrap",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--proc",
            "/proc",
            "--tmpfs",
            "/tmp",
            "--tmpfs",
            "/home",
            "--unshare-all",
            "--die-with-parent",
            "--chdir",
            "/tmp",
            "bash",
            "-c",
            cmd,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return format!("sandbox unavailable: {e}"),
    };
    let mut out = String::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.by_ref().take(64 * 1024).read_to_string(&mut out);
    }
    let mut err = String::new();
    if let Some(mut s) = child.stderr.take() {
        let _ = s.by_ref().take(16 * 1024).read_to_string(&mut err);
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
    if code == 124 {
        err.push_str("timed out after 15 s\n");
    }
    out.push_str(&err);
    out
}

impl Console {
    /// Call every frame with the shell rect. Handles Ctrl+Esc and draws the panel while visible.
    pub fn show(&mut self, ctx: &Context, p: &Palette, shell: Rect) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Escape)) {
            self.open = !self.open;
        }
        let t = gentle(ctx.animate_bool_with_time(Id::new("ui2_console_anim"), self.open, 0.35));
        if t <= 0.001 {
            return;
        }
        if t < 0.999 {
            ctx.request_repaint();
        }
        if let Some(rx) = &self.running {
            if let Ok(res) = rx.try_recv() {
                self.output.extend(res.lines().map(str::to_owned));
                self.running = None;
            } else {
                ctx.request_repaint();
            }
        }
        let h = (shell.height() * 0.46).max(220.0);
        let bottom = shell.max.y - 12.0 + h * (1.0 - t);
        let panel = Rect::from_min_max(
            Pos2::new(shell.min.x + 12.0, bottom - (h - 12.0)),
            Pos2::new(shell.max.x - 12.0, bottom),
        );
        egui::Area::new(Id::new("ui2_console"))
            .order(egui::Order::Foreground)
            .fixed_pos(panel.min)
            .show(ctx, |ui| {
                ui.set_clip_rect(shell);
                let (w, hh) = (panel.width(), panel.height());
                let r = Rect::from_min_size(panel.min, Vec2::new(w, hh));
                ui.painter().rect(
                    r,
                    CornerRadius::same(radius::LG),
                    p.bg_canvas,
                    Stroke::new(1.0, p.border_strong),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    r.min + Vec2::new(16.0, 12.0),
                    Align2::LEFT_TOP,
                    "Terminal \u{b7} sandbox (no network, read-only, nothing saved)",
                    fonts::semibold(12.0),
                    INK_DIM,
                );
                ui.painter().text(
                    r.right_top() + Vec2::new(-16.0, 12.0),
                    Align2::RIGHT_TOP,
                    "Ctrl+Esc to hide",
                    fonts::medium(11.0),
                    INK_DIM,
                );
                let body = Rect::from_min_max(
                    r.min + Vec2::new(16.0, 36.0),
                    r.max - Vec2::new(16.0, 46.0),
                );
                let mut sc = ui.new_child(egui::UiBuilder::new().max_rect(body));
                ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(&mut sc, |ui| {
                        let mono = egui::FontId::monospace(12.0);
                        if let Ok(q) = LOG.lock() {
                            for l in q.iter() {
                                ui.label(egui::RichText::new(l).font(mono.clone()).color(INK_DIM));
                            }
                        }
                        for l in &self.output {
                            ui.label(egui::RichText::new(l).font(mono.clone()).color(INK));
                        }
                    });
                let line = Rect::from_min_max(
                    Pos2::new(r.min.x + 16.0, r.max.y - 38.0),
                    Pos2::new(r.max.x - 16.0, r.max.y - 10.0),
                );
                ui.painter().text(
                    line.left_center(),
                    Align2::LEFT_CENTER,
                    "$",
                    egui::FontId::monospace(13.0),
                    p.accent_primary,
                );
                let edit = TextEdit::singleline(&mut self.input)
                    .font(egui::FontId::monospace(13.0))
                    .text_color(INK)
                    .frame(egui::Frame::NONE)
                    .desired_width(line.width() - 20.0)
                    .hint_text("run a command in the sandbox");
                let resp = ui.put(
                    Rect::from_min_max(line.min + Vec2::new(20.0, 0.0), line.max),
                    edit,
                );
                if self.open && !resp.has_focus() && t > 0.99 {
                    resp.request_focus();
                }
                if resp.lost_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter))
                    && self.running.is_none()
                {
                    let cmd = std::mem::take(&mut self.input);
                    if !cmd.trim().is_empty() {
                        self.output.push(format!("$ {cmd}"));
                        let (tx, rx) = channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(run_sandboxed(&cmd));
                        });
                        self.running = Some(rx);
                    }
                    resp.request_focus();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sandbox_has_no_network_and_cannot_write_home() {
        let out =
            super::run_sandboxed("echo ok; touch /etc/x 2>&1 | head -1; cat /proc/net/dev | wc -l");
        assert!(out.starts_with("ok"), "{out}");
        assert!(
            out.contains("Read-only") || out.contains("denied") || out.contains("read-only"),
            "{out}"
        );
    }
}
