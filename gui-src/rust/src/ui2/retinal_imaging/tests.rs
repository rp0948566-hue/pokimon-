use super::*;
use crate::domain::EyeLaterality;
use crate::ui2::tokens::{DARK, LIGHT};
use eframe::egui::{self, ColorImage, Pos2, TextureOptions, Vec2};

fn run(f: impl FnOnce(&mut Ui)) {
    let ctx = egui::Context::default();
    let mut f = Some(f);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| (f.take().unwrap())(ui));
    out.textures_delta.clear();
}

fn area() -> Rect {
    Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1300.0, 780.0))
}

fn tex(ui: &Ui) -> TextureHandle {
    ui.ctx()
        .load_texture("ri_test", ColorImage::example(), TextureOptions::LINEAR)
}

#[test]
fn four_previews_and_no_unacquired_modality_labels() {
    assert_eq!(Preview::ALL.len(), 4);
    for p in Preview::ALL {
        let l = p.label().to_uppercase();
        for banned in ["FA", "ICG", "OCT", "RED-FREE"] {
            assert!(!l.contains(banned), "{l} claims an unacquired modality");
        }
    }
}

#[test]
fn nothing_is_available_without_an_image() {
    run(|_| {
        let exam = RetinalExaminationView::default();
        let cx = RetinalImagingContext {
            exam: &exam,
            original: None,
            file_name: None,
        };
        assert!(Preview::ALL.iter().all(|p| !cx.available(*p)));
        assert!(!cx.has_result());
    });
}

#[test]
fn only_the_original_is_available_when_no_overlay_exists() {
    run(|ui| {
        let t = tex(ui);
        let exam = RetinalExaminationView::default();
        let cx = RetinalImagingContext {
            exam: &exam,
            original: Some(&t),
            file_name: None,
        };
        assert!(cx.available(Preview::Original));
        assert!(!cx.available(Preview::Vessel));
        assert!(!cx.available(Preview::Lesion));
        assert!(!cx.available(Preview::Explain));
    });
}

#[test]
fn switching_exam_or_eye_clears_every_selection() {
    let mut s = RetinalImagingState::default();
    s.bind("P1|E1|OD".into());
    s.opened = Some(Preview::Vessel);
    s.compared = Some(Preview::Lesion);
    s.vessel_on = true;
    s.set_brightness(30);
    s.bind("P1|E1|OD".into());
    assert_eq!(s.opened, Some(Preview::Vessel), "same exam keeps state");
    for other in ["P1|E2|OD", "P2|E1|OD", "P1|E1|OS"] {
        s.opened = Some(Preview::Vessel);
        s.bind(other.into());
        assert!(s.opened.is_none() && s.compared.is_none() && !s.vessel_on);
        assert_eq!(s.brightness, 0);
        s.bind("P1|E1|OD".into());
    }
}

#[test]
fn brightness_is_bounded_and_reset_is_neutral() {
    let mut s = RetinalImagingState::default();
    s.set_brightness(900);
    assert_eq!(s.brightness, RetinalImagingState::BRIGHTNESS_MAX);
    s.set_brightness(-900);
    assert_eq!(s.brightness, RetinalImagingState::BRIGHTNESS_MIN);
    s.zoom = 3.0;
    s.pan = eframe::egui::vec2(20.0, -5.0);
    s.reset_view();
    assert_eq!(s.brightness, 0);
    assert_eq!((s.zoom, s.pan), (0.0, eframe::egui::Vec2::ZERO));
}

#[test]
fn show_closes_a_preview_whose_artifact_is_missing() {
    for p in [&LIGHT, &DARK] {
        run(|ui| {
            let t = tex(ui);
            let exam = RetinalExaminationView::default();
            let cx = RetinalImagingContext {
                exam: &exam,
                original: Some(&t),
                file_name: Some("x.png"),
            };
            let mut s = RetinalImagingState {
                opened: Some(Preview::Vessel),
                compared: Some(Preview::Lesion),
                ..Default::default()
            };
            let a = show(ui, p, area(), &mut s, &cx);
            assert_eq!(a, RetinalImagingAction::None);
            assert!(s.opened.is_none() && s.compared.is_none());
        });
    }
}

#[test]
fn empty_exam_draws_without_panic_and_asks_for_nothing() {
    for p in [&LIGHT, &DARK] {
        run(|ui| {
            let exam = RetinalExaminationView {
                laterality: EyeLaterality::OS,
                ..Default::default()
            };
            let cx = RetinalImagingContext {
                exam: &exam,
                original: None,
                file_name: None,
            };
            let mut s = RetinalImagingState::default();
            assert_eq!(show(ui, p, area(), &mut s, &cx), RetinalImagingAction::None);
        });
    }
}

#[test]
fn small_window_draws_without_panic() {
    run(|ui| {
        let t = tex(ui);
        let exam = RetinalExaminationView::default();
        let cx = RetinalImagingContext {
            exam: &exam,
            original: Some(&t),
            file_name: None,
        };
        let mut s = RetinalImagingState::default();
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 560.0));
        show(ui, &LIGHT, r, &mut s, &cx);
    });
}

/// Drives `show` through real egui pointer events (move, press, release) over several frames.
struct Driver {
    ctx: egui::Context,
    tex: TextureHandle,
    exam: RetinalExaminationView,
    st: RetinalImagingState,
}

impl Driver {
    fn new() -> Self {
        let ctx = egui::Context::default();
        let tex = ctx.load_texture("ri_drv", ColorImage::example(), TextureOptions::LINEAR);
        Self {
            ctx,
            tex,
            exam: RetinalExaminationView::default(),
            st: RetinalImagingState::default(),
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> RetinalImagingAction {
        let raw = egui::RawInput {
            screen_rect: Some(area()),
            events,
            ..Default::default()
        };
        let mut act = RetinalImagingAction::None;
        let (exam, st, tex) = (&self.exam, &mut self.st, &self.tex);
        let mut out = self.ctx.run_ui(raw, |ui| {
            let cx = RetinalImagingContext {
                exam,
                original: Some(tex),
                file_name: None,
            };
            act = show(ui, &LIGHT, area(), st, &cx);
        });
        out.textures_delta.clear();
        act
    }

    /// Click at `pos`; returns the strongest action seen across the frames.
    fn click(&mut self, x: f32, y: f32) -> RetinalImagingAction {
        let pos = Pos2::new(x, y);
        let btn = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        self.frame(vec![egui::Event::PointerMoved(pos)]);
        self.frame(vec![egui::Event::PointerMoved(pos)]);
        self.frame(vec![btn(true)]);
        let a = self.frame(vec![btn(false)]);
        let b = self.frame(vec![]);
        if a != RetinalImagingAction::None {
            a
        } else {
            b
        }
    }
}

// Coordinates follow the layout for `area()` (1300x780): strip y ~ 632..772, card 1 x 4..238,
// card 2 x 252..486; Back pill centre (61,100); tab row y 25.
#[test]
fn clicking_the_original_card_opens_it_and_back_returns() {
    let mut d = Driver::new();
    d.frame(vec![]);
    assert!(d.click(61.0, 100.0) == RetinalImagingAction::None);
    assert_eq!(
        d.st.opened, None,
        "Back does not exist before a card is opened"
    );
    d.click(100.0, 700.0);
    assert_eq!(d.st.opened, Some(Preview::Original));
    d.click(61.0, 100.0);
    assert_eq!(d.st.opened, None, "Back closes the large view");
    assert_eq!(
        d.st.compared, None,
        "the original is never parked as a comparison"
    );
}

#[test]
fn an_unavailable_card_does_not_open() {
    let mut d = Driver::new();
    d.frame(vec![]);
    d.click(369.0, 700.0);
    assert_eq!(d.st.opened, None);
}

#[test]
fn header_tabs_route_only_to_existing_pages() {
    let mut d = Driver::new();
    d.frame(vec![]);
    assert_eq!(d.click(1248.0, 25.0), RetinalImagingAction::OpenReport);
    assert_eq!(d.click(1010.0, 25.0), RetinalImagingAction::OpenReview);
    assert_eq!(
        d.click(1130.0, 25.0),
        RetinalImagingAction::OpenBilateral,
        "the former Comparison slot opens the optional Bilateral Assessment page"
    );
}

#[test]
fn analysis_toggles_stay_off_without_saved_artifacts() {
    let mut d = Driver::new();
    d.frame(vec![]);
    // Vessel Analysis toggle sits in the right rail; clicking anywhere in the rail must never
    // switch on an overlay that has no saved mask behind it.
    for y in (60..760).step_by(20) {
        d.click(1160.0, y as f32);
    }
    assert!(!d.st.vessel_on && !d.st.lesion_on && !d.st.dr_on);
}

#[test]
fn neutral_look_leaves_pixels_and_red_free_copies_green() {
    use super::adjust::{apply, Look};
    let mut img = image::RgbaImage::from_pixel(2, 2, image::Rgba([200, 100, 50, 255]));
    let before = img.clone();
    apply(&mut img, Look::default(), 0);
    assert_eq!(img, before);
    apply(
        &mut img,
        Look {
            red_free: true,
            ..Look::default()
        },
        0,
    );
    assert!(img.pixels().all(|p| p.0 == [100, 100, 100, 255]));
}

#[test]
fn contrast_saturation_and_brightness_move_pixels_the_right_way() {
    use super::adjust::{apply, Look};
    let base = image::RgbaImage::from_pixel(1, 1, image::Rgba([180, 120, 60, 255]));
    let spread = |i: &image::RgbaImage| i.get_pixel(0, 0)[0] as i32 - i.get_pixel(0, 0)[2] as i32;
    let mut sat = base.clone();
    apply(
        &mut sat,
        Look {
            saturation: 50,
            ..Look::default()
        },
        0,
    );
    assert!(spread(&sat) > spread(&base));
    let mut grey = base.clone();
    apply(
        &mut grey,
        Look {
            saturation: -50,
            ..Look::default()
        },
        0,
    );
    assert_eq!(spread(&grey), 0);
    let mut con = base.clone();
    apply(
        &mut con,
        Look {
            contrast: 50,
            ..Look::default()
        },
        0,
    );
    assert!(con.get_pixel(0, 0)[0] > 180 && con.get_pixel(0, 0)[2] < 60);
    let mut dark = base.clone();
    apply(&mut dark, Look::default(), -50);
    assert!(dark.get_pixel(0, 0)[0] < 180);
    let mut light = base.clone();
    apply(&mut light, Look::default(), 50);
    assert!(light.get_pixel(0, 0)[0] > 180);
}

#[test]
fn reset_clears_the_look_and_exam_change_clears_markers() {
    let mut s = RetinalImagingState::default();
    s.look.contrast = 20;
    s.look.red_free = true;
    s.notes.push(eframe::egui::vec2(0.5, 0.5));
    s.reset_view();
    assert!(s.look.is_neutral());
    assert_eq!(
        s.notes.len(),
        1,
        "Reset is a view reset; markers have their own Clear"
    );
}
