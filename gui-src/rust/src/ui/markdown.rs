//! Chat reply renderer: tight text, lists, tables, bar charts, step flows and inline source pills.
//! The reply is untrusted model text; unknown markup is dropped, never executed.

use crate::domain::WebSite;
use crate::ui2::fonts;
use eframe::egui::{
    self, text::LayoutJob, Color32, CornerRadius, FontFamily, FontId, Margin, Pos2, Rect, RichText,
    Stroke, TextFormat, Vec2,
};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatFont {
    Sans,
    Serif,
    Mono,
    Cursive,
}

impl ChatFont {
    pub const ALL: [(ChatFont, &'static str); 4] = [
        (ChatFont::Sans, "Sans"),
        (ChatFont::Serif, "Serif"),
        (ChatFont::Mono, "Mono"),
        (ChatFont::Cursive, "Cursive"),
    ];

    fn id(self, size: f32, bold: bool) -> FontId {
        // Named families exist only after the design fonts are installed.
        if !fonts::INSTALLED.load(Ordering::Relaxed) {
            return FontId::new(
                size,
                if self == Self::Mono {
                    FontFamily::Monospace
                } else {
                    FontFamily::Proportional
                },
            );
        }
        let family = match self {
            Self::Sans if bold => FontFamily::Name(fonts::SEMIBOLD.into()),
            Self::Sans => FontFamily::Proportional,
            Self::Serif => FontFamily::Name(fonts::SERIF.into()),
            Self::Mono => FontFamily::Monospace,
            Self::Cursive => FontFamily::Name(fonts::CURSIVE.into()),
        };
        FontId::new(size, family)
    }
}

/// Reader's choices; the AI may only pick among these through `[[name|text]]` tags.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChatStyle {
    pub font: ChatFont,
    pub size: f32,
}

impl Default for ChatStyle {
    fn default() -> Self {
        Self {
            font: ChatFont::Sans,
            size: 14.5,
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    color: Option<Color32>,
    font: Option<ChatFont>,
}

fn tag_style(name: &str) -> Option<(Option<Color32>, Option<ChatFont>)> {
    Some(match name.trim().to_lowercase().as_str() {
        "serif" => (None, Some(ChatFont::Serif)),
        "sans" => (None, Some(ChatFont::Sans)),
        "mono" => (None, Some(ChatFont::Mono)),
        "cursive" | "hand" => (None, Some(ChatFont::Cursive)),
        "green" => (Some(Color32::from_rgb(86, 200, 120)), None),
        "red" => (Some(Color32::from_rgb(240, 110, 110)), None),
        "blue" => (Some(Color32::from_rgb(100, 170, 255)), None),
        "gold" => (Some(Color32::from_rgb(222, 178, 80)), None),
        "gray" | "grey" => (Some(Color32::from_rgb(150, 150, 150)), None),
        _ => return None,
    })
}

/// Inline spans plus the `[W#]` source references found in the text (zero-based, in order, no repeats).
fn parse_inline(s: &str) -> (Vec<Span>, Vec<usize>) {
    let c: Vec<char> = s.chars().collect();
    let (mut spans, mut refs): (Vec<Span>, Vec<usize>) = (Vec::new(), Vec::new());
    let (mut bold, mut italic, mut code) = (false, false, false);
    let mut tag: Option<(Option<Color32>, Option<ChatFont>)> = None;
    let mut buf = String::new();
    let mut i = 0;
    let flush = |buf: &mut String,
                 spans: &mut Vec<Span>,
                 bold,
                 italic,
                 code,
                 tag: Option<(Option<Color32>, Option<ChatFont>)>| {
        if !buf.is_empty() {
            spans.push(Span {
                text: std::mem::take(buf),
                bold,
                italic,
                code,
                color: tag.and_then(|t| t.0),
                font: tag.and_then(|t| t.1),
            });
        }
    };
    while i < c.len() {
        let rest: String = c[i..c.len().min(i + 12)].iter().collect();
        if rest.starts_with("[[") {
            if let Some(bar) = c[i + 2..].iter().take(12).position(|&x| x == '|') {
                let name: String = c[i + 2..i + 2 + bar].iter().collect();
                flush(&mut buf, &mut spans, bold, italic, code, tag);
                tag = tag_style(&name);
                i += 2 + bar + 1;
                continue;
            }
        }
        if rest.starts_with("]]") {
            flush(&mut buf, &mut spans, bold, italic, code, tag);
            tag = None;
            i += 2;
            continue;
        }
        if c[i] == '[' && c.get(i + 1) == Some(&'W') {
            let digits: String = c[i + 2..]
                .iter()
                .take_while(|x| x.is_ascii_digit())
                .collect();
            if !digits.is_empty() && c.get(i + 2 + digits.len()) == Some(&']') {
                if let Ok(n) = digits.parse::<usize>() {
                    if n > 0 && !refs.contains(&(n - 1)) {
                        refs.push(n - 1);
                    }
                }
                i += 3 + digits.len();
                continue;
            }
        }
        if rest.starts_with("**") {
            flush(&mut buf, &mut spans, bold, italic, code, tag);
            bold = !bold;
            i += 2;
            continue;
        }
        if c[i] == '`' {
            flush(&mut buf, &mut spans, bold, italic, code, tag);
            code = !code;
            i += 1;
            continue;
        }
        let spaced = |j: Option<&char>| j.is_none_or(|x| x.is_whitespace());
        if c[i] == '*'
            && !code
            && (italic || !spaced(c.get(i + 1)))
            && (!italic || !spaced(i.checked_sub(1).and_then(|j| c.get(j))))
        {
            flush(&mut buf, &mut spans, bold, italic, code, tag);
            italic = !italic;
            i += 1;
            continue;
        }
        buf.push(c[i]);
        i += 1;
    }
    flush(&mut buf, &mut spans, bold, italic, code, tag);
    (spans, refs)
}

struct Palette {
    text: Color32,
    muted: Color32,
    line: Color32,
    fill: Color32,
    accent: Color32,
}

fn palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode {
        // Snow text on the dark canvas.
        Palette {
            text: Color32::from_rgb(236, 238, 240),
            muted: Color32::from_rgb(150, 154, 160),
            line: Color32::from_rgb(52, 55, 60),
            fill: Color32::from_rgb(30, 32, 35),
            accent: Color32::from_rgb(86, 200, 120),
        }
    } else {
        // Smoky black on the light canvas.
        Palette {
            text: Color32::from_rgb(40, 42, 46),
            muted: Color32::from_rgb(110, 114, 120),
            line: Color32::from_rgb(215, 217, 220),
            fill: Color32::from_rgb(244, 245, 246),
            accent: Color32::from_rgb(30, 140, 80),
        }
    }
}

fn job(
    spans: &[Span],
    style: ChatStyle,
    size: f32,
    pal: &Palette,
    wrap: f32,
    force_bold: bool,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    for s in spans {
        let font = s.font.unwrap_or(style.font);
        let bold = s.bold || force_bold;
        let id = if s.code {
            ChatFont::Mono.id(size * 0.92, false)
        } else {
            font.id(size, bold)
        };
        let mut color = s
            .color
            .unwrap_or(if bold { brighten(pal.text) } else { pal.text });
        if s.code {
            color = pal.accent;
        }
        job.append(
            &s.text,
            0.0,
            TextFormat {
                font_id: id,
                color,
                italics: s.italic,
                line_height: Some(size * 1.5),
                background: if s.code {
                    pal.fill
                } else {
                    Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    job
}

fn brighten(c: Color32) -> Color32 {
    // Bold text is a shade stronger than body text in either theme.
    if c.r() > 128 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

fn pill(ui: &mut egui::Ui, site: &WebSite, pal: &Palette) -> egui::Response {
    let label = RichText::new(&site.site).size(11.0).color(pal.muted);
    ui.add(
        egui::Button::new(label)
            .fill(pal.fill)
            .stroke(Stroke::new(1.0, pal.line))
            .corner_radius(CornerRadius::same(9))
            .min_size(Vec2::new(0.0, 18.0)),
    )
    .on_hover_text(&site.title)
}

/// One paragraph: wrapped text with the source pills right after its last word.
fn paragraph(
    ui: &mut egui::Ui,
    text: &str,
    style: ChatStyle,
    size: f32,
    pal: &Palette,
    sites: &[WebSite],
    force_bold: bool,
) -> Option<usize> {
    let (spans, refs) = parse_inline(text);
    let mut clicked = None;
    let wrap = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(5.0, 2.0);
        if !spans.is_empty() {
            ui.label(job(&spans, style, size, pal, wrap, force_bold));
        }
        for r in refs {
            if let Some(site) = sites.get(r) {
                if pill(ui, site, pal).clicked() {
                    clicked = Some(r);
                }
            }
        }
    });
    clicked
}

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn table(ui: &mut egui::Ui, rows: &[Vec<String>], style: ChatStyle, pal: &Palette) {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return;
    }
    // Frame margin (16) and the gaps between columns (14) come off the width before it is shared out.
    let col_w =
        ((ui.available_width() - 16.0 - 14.0 * (cols as f32 - 1.0)) / cols as f32).max(60.0);
    egui::Frame::new()
        .stroke(Stroke::new(1.0, pal.line))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .num_columns(cols)
                .spacing([14.0, 5.0])
                .show(ui, |ui| {
                    for (r, row) in rows.iter().enumerate() {
                        for c in 0..cols {
                            let (spans, _) = parse_inline(row.get(c).map_or("", String::as_str));
                            ui.label(job(&spans, style, style.size - 1.0, pal, col_w, r == 0));
                        }
                        ui.end_row();
                    }
                });
        });
}

/// `label: value` lines as horizontal bars; values are shown as written.
fn chart(ui: &mut egui::Ui, lines: &[&str], style: ChatStyle, pal: &Palette) {
    let rows: Vec<(String, f32, String)> = lines
        .iter()
        .filter_map(|l| {
            let (label, value) = l.rsplit_once(':').or_else(|| l.rsplit_once('|'))?;
            let n: f32 = value
                .trim()
                .trim_end_matches('%')
                .replace(',', "")
                .parse()
                .ok()?;
            Some((
                label.trim().to_string(),
                n.max(0.0),
                value.trim().to_string(),
            ))
        })
        .collect();
    let max = rows.iter().map(|r| r.1).fold(0.0, f32::max).max(1.0);
    let colors = [
        Color32::from_rgb(40, 130, 160),
        Color32::from_rgb(60, 170, 140),
        Color32::from_rgb(200, 155, 80),
        Color32::from_rgb(150, 110, 190),
    ];
    let full = (ui.available_width() * 0.55).max(120.0);
    for (i, (label, n, shown)) in rows.iter().enumerate() {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(full, 20.0), egui::Sense::hover());
            let bar = Rect::from_min_size(rect.min, Vec2::new((full * n / max).max(4.0), 20.0));
            ui.painter().rect_filled(bar, 6.0, colors[i % colors.len()]);
            ui.label(
                RichText::new(format!("{shown} {label}"))
                    .font(style.font.id(style.size - 1.5, false))
                    .color(pal.muted),
            );
        });
        ui.add_space(2.0);
    }
}

/// `title | subtitle` lines as boxes joined by arrows.
fn steps(ui: &mut egui::Ui, lines: &[&str], style: ChatStyle, pal: &Palette) {
    let w = ui.available_width().min(560.0);
    for (i, line) in lines.iter().filter(|l| !l.trim().is_empty()).enumerate() {
        if i > 0 {
            ui.allocate_ui(Vec2::new(w, 14.0), |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("\u{2193}").size(12.0).color(pal.muted))
                });
            });
        }
        let (title, sub) = line.split_once('|').unwrap_or((line, ""));
        egui::Frame::new()
            .stroke(Stroke::new(1.0, pal.line))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 7))
            .show(ui, |ui| {
                ui.set_width(w - 26.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(title.trim())
                            .font(style.font.id(style.size, true))
                            .color(pal.text),
                    );
                    if !sub.trim().is_empty() {
                        ui.label(
                            RichText::new(sub.trim())
                                .font(style.font.id(style.size - 2.0, false))
                                .color(pal.muted),
                        );
                    }
                });
            });
    }
}

/// Draws the reply; returns the index of a source pill the reader clicked.
pub fn render(ui: &mut egui::Ui, text: &str, style: ChatStyle, sites: &[WebSite]) -> Option<usize> {
    let pal = palette(ui);
    let mut clicked = None;
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    ui.spacing_mut().item_spacing.y = 4.0;
    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim();
        if let Some(lang) = line.strip_prefix("```") {
            let end = lines[i + 1..]
                .iter()
                .position(|l| l.trim().starts_with("```"));
            let body = &lines[i + 1..end.map_or(lines.len(), |e| i + 1 + e)];
            match lang.trim() {
                "chart" => chart(ui, body, style, &pal),
                "steps" | "flow" => steps(ui, body, style, &pal),
                _ => {
                    egui::Frame::new()
                        .fill(pal.fill)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(body.join("\n"))
                                    .font(ChatFont::Mono.id(style.size - 1.5, false))
                                    .color(pal.text),
                            );
                        });
                }
            }
            ui.add_space(4.0);
            i += end.map_or(lines.len() - i, |e| e + 2);
            continue;
        }
        if line.starts_with('|') {
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                let row = split_row(lines[i]);
                if !row
                    .iter()
                    .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
                {
                    rows.push(row);
                }
                i += 1;
            }
            table(ui, &rows, style, &pal);
            ui.add_space(4.0);
            continue;
        }
        i += 1;
        if line.is_empty() {
            ui.add_space(2.0);
        } else if line.chars().all(|c| matches!(c, '-' | '_' | '*')) && line.len() >= 3 {
            ui.separator();
        } else if let Some(h) = line
            .trim_start_matches('#')
            .strip_prefix(' ')
            .filter(|_| line.starts_with('#'))
        {
            ui.add_space(4.0);
            clicked = clicked.or(paragraph(ui, h, style, style.size + 1.5, &pal, sites, true));
        } else if let Some(item) = ["- ", "* ", "\u{2022} "]
            .iter()
            .find_map(|p| line.strip_prefix(p))
        {
            ui.horizontal_top(|ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("\u{2022}").size(style.size).color(pal.muted));
                clicked = clicked.or(paragraph(ui, item, style, style.size, &pal, sites, false));
            });
        } else if let Some((num, item)) = line.split_once(['.', ')']).filter(|(n, r)| {
            !n.is_empty()
                && n.len() < 3
                && n.chars().all(|c| c.is_ascii_digit())
                && r.starts_with(' ')
        }) {
            ui.horizontal_top(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("{num}."))
                        .size(style.size)
                        .color(pal.muted),
                );
                clicked = clicked.or(paragraph(
                    ui,
                    item.trim_start(),
                    style,
                    style.size,
                    &pal,
                    sites,
                    false,
                ));
            });
        } else if let Some(q) = line.strip_prefix("> ") {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(3.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 1.5, pal.line);
                clicked = clicked.or(paragraph(ui, q, style, style.size, &pal, sites, false));
            });
        } else {
            clicked = clicked.or(paragraph(ui, line, style, style.size, &pal, sites, false));
        }
    }
    let _ = Pos2::ZERO;
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_styles_and_refs() {
        let (spans, refs) = parse_inline("Eat **fish** and *nuts*[W2] [[green|daily]] `x`");
        assert_eq!(refs, vec![1]);
        assert!(spans.iter().any(|s| s.text == "fish" && s.bold));
        assert!(spans.iter().any(|s| s.text == "nuts" && s.italic));
        assert!(spans.iter().any(|s| s.text == "daily" && s.color.is_some()));
        assert!(spans.iter().any(|s| s.text == "x" && s.code));
    }

    #[test]
    fn unknown_tag_keeps_text_and_drops_markup() {
        let (spans, _) = parse_inline("[[evil|hello]] world");
        let all: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(all, "hello world");
        assert!(spans.iter().all(|s| s.color.is_none() && s.font.is_none()));
    }

    #[test]
    fn lone_star_is_literal_and_ref_zero_is_ignored() {
        let (spans, refs) = parse_inline("2 * 3 [W0]");
        assert!(refs.is_empty());
        assert!(spans.iter().all(|s| !s.italic));
    }
}
