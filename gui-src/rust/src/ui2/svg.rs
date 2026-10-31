//! Paints the Figma icon paths (absolute M L H V C Z) as egui strokes.
use eframe::egui::{
    epaint::{CubicBezierShape, PathShape},
    Color32, Painter, Pos2, Rect, Shape, Stroke,
};

enum Tok {
    Cmd(char),
    Num(f32),
}

fn tokens(d: &str) -> Vec<Tok> {
    let (mut out, mut cur) = (Vec::new(), String::new());
    let flush = |cur: &mut String, out: &mut Vec<Tok>| {
        if let Ok(v) = cur.parse() {
            out.push(Tok::Num(v));
        }
        cur.clear();
    };
    for ch in d.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' => {
                flush(&mut cur, &mut out);
                out.push(Tok::Cmd(ch));
            }
            '-' => {
                flush(&mut cur, &mut out);
                cur.push(ch);
            }
            '.' if cur.contains('.') => {
                flush(&mut cur, &mut out);
                cur.push(ch);
            }
            ' ' | ',' => flush(&mut cur, &mut out),
            _ => cur.push(ch),
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Flattens a path into polylines (closed flag per subpath) in viewBox units.
pub fn flatten(d: &str) -> Vec<(Vec<Pos2>, bool)> {
    let t = tokens(d);
    let (mut i, mut cmd) = (0, ' ');
    let (mut subs, mut pts): (Vec<(Vec<Pos2>, bool)>, Vec<Pos2>) = (Vec::new(), Vec::new());
    let mut cur = Pos2::ZERO;
    let num = |i: &mut usize| -> Option<f32> {
        if let Some(Tok::Num(v)) = t.get(*i) {
            *i += 1;
            Some(*v)
        } else {
            None
        }
    };
    while i < t.len() {
        if let Tok::Cmd(c) = t[i] {
            cmd = c;
            i += 1;
            if c == 'Z' {
                if pts.len() > 1 {
                    subs.push((std::mem::take(&mut pts), true));
                }
                continue;
            }
        }
        match cmd {
            'M' | 'L' => {
                let (Some(x), Some(y)) = (num(&mut i), num(&mut i)) else {
                    break;
                };
                cur = Pos2::new(x, y);
                if cmd == 'M' {
                    if pts.len() > 1 {
                        subs.push((std::mem::take(&mut pts), false));
                    }
                    pts.clear();
                    cmd = 'L';
                }
                pts.push(cur);
            }
            'H' => {
                let Some(x) = num(&mut i) else { break };
                cur.x = x;
                pts.push(cur);
            }
            'V' => {
                let Some(y) = num(&mut i) else { break };
                cur.y = y;
                pts.push(cur);
            }
            'C' => {
                let v: Vec<f32> = (0..6).filter_map(|_| num(&mut i)).collect();
                if v.len() < 6 {
                    break;
                }
                let p = |k: usize| Pos2::new(v[k], v[k + 1]);
                let b = CubicBezierShape::from_points_stroke(
                    [cur, p(0), p(2), p(4)],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::NONE,
                );
                pts.extend(b.flatten(Some(0.05)).into_iter().skip(1));
                cur = p(4);
            }
            _ => break,
        }
    }
    if pts.len() > 1 {
        subs.push((pts, false));
    }
    subs
}

/// Paints `paths` scaled from a `view`-unit square into `rect`.
pub fn paint_paths(
    painter: &Painter,
    paths: &[&str],
    view: f32,
    rect: Rect,
    color: Color32,
    width: f32,
) {
    let k = rect.width() / view;
    let stroke = Stroke::new(width * k, color);
    for d in paths {
        for (pts, closed) in flatten(d) {
            let pts: Vec<Pos2> = pts.iter().map(|p| rect.min + p.to_vec2() * k).collect();
            painter.add(Shape::Path(if closed {
                PathShape::closed_line(pts, stroke)
            } else {
                PathShape::line(pts, stroke)
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_home_icon_to_one_closed_outline() {
        let s = flatten("M2.25 8.25L9 3L15.75 8.25V15Z");
        assert_eq!(s.len(), 1);
        assert!(s[0].1);
        assert_eq!(s[0].0.len(), 4);
        assert_eq!(s[0].0[3], Pos2::new(15.75, 15.0));
    }

    #[test]
    fn splits_subpaths_and_flattens_curves() {
        let s = flatten("M0 0C0 1 1 1 1 0M5 5H6");
        assert_eq!(s.len(), 2);
        assert!(s[0].0.len() > 3);
        assert_eq!(s[1].0, vec![Pos2::new(5.0, 5.0), Pos2::new(6.0, 5.0)]);
    }
}
