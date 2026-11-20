use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

// The window draws a frame in this time and no sooner (`lib.rs` holds it to that), whatever the screen's own pace.
pub const FRAME: f32 = 1.0 / 60.0;
// The share of a frame the graphics card may spend on a picture: the rest is the window's and the system's.
// While the window is used (a button or a finger held, the wheel, a text typed, a pane in motion) it is the
// smaller share.
pub const SHARE: f32 = 0.5;
pub const SHARE_ATTENDED: f32 = 0.25;
// A batch that overran its slice is followed by one cut to the slice at once; one that ran short by one that
// aims at this share of the slice, and is at most so many times larger.
pub const AIM: f64 = 0.8;
const MOST: f64 = 8.0;

const _: () = assert!(SHARE < 0.6 && SHARE_ATTENDED <= SHARE * 0.5);

static ATTENDED: AtomicBool = AtomicBool::new(false);

pub fn attend(used: bool) {
    ATTENDED.store(used, Ordering::Relaxed);
}

// The seconds one batch of a picture may keep the graphics card: one batch is sent a frame, and none before the
// one before it has come back, so the card never has more than this ahead of the window's own frame.
pub fn slice() -> f32 {
    slice_of(ATTENDED.load(Ordering::Relaxed))
}

pub fn slice_of(attended: bool) -> f32 {
    FRAME * if attended { SHARE_ATTENDED } else { SHARE }
}

// The time shown while a picture is on its way: the seconds since it was asked for, a tenth at a step, and
// nothing before the first tenth. It never passes the time the finished picture is shown with.
pub fn counted(seconds: f32) -> Option<f32> {
    (seconds >= 0.1).then(|| (seconds * 10.0).floor() / 10.0)
}

// The size of the next batch, from the size of the last and the seconds it took.
pub fn next(sent: f64, took: f32, slice: f32) -> f64 {
    let fits = f64::from(slice) / f64::from(took.max(1e-5));
    if fits < 1.0 {
        sent * fits
    } else {
        sent * (AIM * fits).clamp(1.0, MOST)
    }
}

// Whether what the window was told in a frame is the window being used: a button or a finger held, the wheel, a
// pinch, a key, a text. The pointer moving over the window, or leaving it, is not.
pub fn used(held: bool, events: &[egui::Event]) -> bool {
    let idle = |event: &egui::Event| {
        matches!(
            event,
            egui::Event::PointerMoved(_)
                | egui::Event::MouseMoved(_)
                | egui::Event::PointerGone
                | egui::Event::WindowFocused(_)
        )
    };
    held || events.iter().any(|event| !idle(event))
}

// A view that moves is drawn anew for every frame, small, and shown at the picture's size until the view rests:
// a draft. It is this many times smaller a side, by how long the card takes for it.
pub const DRAFT_SIDES: [u32; 6] = [2, 3, 4, 6, 8, 12];
const DRAFT_FIRST: usize = 2;
// The shorter side of a draft is never under this many pixels: 320 × 180 of a picture of 16 to 9.
pub const DRAFT_LEAST: u32 = 180;
// A draft is drawn this many times wider and higher than the view, so that the frames it is late by show the
// picture too, and nothing beside it.
pub const DRAFT_MARGIN: f64 = 1.25;
// One sample a pixel, and none drawn again.
pub const DRAFT_SAMPLES: u32 = 1;
// A draft is made finer when so many more than not would have fit in this share of the slice at the finer size.
const DRAFT_CALM: u32 = 8;
const DRAFT_ROOM: f32 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draft {
    step: usize,
    calm: u32,
}

impl Draft {
    pub const fn new() -> Self {
        Self {
            step: DRAFT_FIRST,
            calm: 0,
        }
    }

    fn coarsest(shorter: u32) -> usize {
        DRAFT_SIDES
            .iter()
            .rposition(|side| shorter / side >= DRAFT_LEAST)
            .unwrap_or(0)
    }

    // How many times smaller a side than a picture with this shorter side.
    pub fn side(&self, shorter: u32) -> u32 {
        DRAFT_SIDES[self.step.min(Self::coarsest(shorter))]
    }

    pub fn size(&self, picture: [u32; 2]) -> [u32; 2] {
        let side = f64::from(self.side(picture[0].min(picture[1])));
        picture.map(|pixels| ((f64::from(pixels) * DRAFT_MARGIN / side).round() as u32).max(1))
    }

    // Takes in the seconds the card worked on a draft: one that overran the slice is followed by one small enough
    // to fit, at once; one that would fit a step finer with room to spare, DRAFT_CALM times more often than not, by a
    // finer one.
    pub fn took(&mut self, seconds: f32, slice: f32, shorter: u32) {
        let last = Self::coarsest(shorter);
        let at = self.step.min(last);
        let at_side = |to: usize| {
            let fewer = DRAFT_SIDES[at] as f32 / DRAFT_SIDES[to] as f32;
            seconds * fewer * fewer
        };
        self.step = at;
        if seconds > slice {
            self.step = (at..=last).find(|to| at_side(*to) <= slice).unwrap_or(last);
            self.calm = 0;
        } else if at > 0 && at_side(at - 1) <= slice * DRAFT_ROOM {
            self.calm += 1;
            if self.calm >= DRAFT_CALM {
                (self.step, self.calm) = (at - 1, 0);
            }
        } else {
            self.calm = self.calm.saturating_sub(1);
        }
    }
}

// What is drawn next for a view that is not the picture's, where the card drafts: one draft at a time, each of the
// view as it is when the one before has landed, and the picture itself once the draft on the screen is of the very
// view, which is when the view has stood still for as long as a draft takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Next {
    Draft,
    Wait,
    Picture,
}

pub fn hand_over(flying: bool, shown: bool) -> Next {
    if flying {
        Next::Wait
    } else if shown {
        Next::Picture
    } else {
        Next::Draft
    }
}

// The share of the window a picture does not cover, when the window shows the part of it around [across, along],
// `larger` times larger.
pub fn beside([across, along, larger]: [f32; 3]) -> f32 {
    let half = 0.5 / larger.max(1e-9);
    let covered = |at: f32| ((at + half).min(1.0) - (at - half).max(0.0)).max(0.0) / (2.0 * half);
    1.0 - covered(across) * covered(along)
}

// A view is rendered when its picture is expected within this many seconds; a longer one is the picture the app
// has, magnified.
pub const LONGEST: f64 = 10.0;
// A render is stopped once it has run this long and its progress says it will pass LONGEST.
pub const PROOF: f64 = 1.0;
// A deep view of a flame costs the processor this many times a whole one, until a render has shown how many.
const DEEP_COST: f64 = 10.0;
// Steps of a fractal drawn pixel by pixel the processor walks in the time of one sample of a flame, until a render
// of the kind has shown how many. On a Ryzen 7 5700X mandelbrot-whole walks 67 and mandelbrot-seahorse-valley 205.
const STEPS_A_SAMPLE: f64 = 60.0;
pub const FLAME: &str = "flame";
// A view this many times larger than the scene's own holds an eighth of it: IN_VIEW of the core's deep.rs, from
// where a flame is drawn as a deep view.
pub const DEEP_FROM: f64 = 2.83;

// What a layer costs the processor: samples of a flame or of an attractor, or steps of a fractal drawn pixel by
// pixel.
pub struct Part<'a> {
    pub kind: &'a str,
    pub work: f64,
    pub stepped: bool,
}

// The most work a second the processor has shown in this session, for whole and for deep views of each kind.
#[derive(Default)]
pub struct Speeds(HashMap<(String, bool), f64>);

impl Speeds {
    pub fn seen(&mut self, kind: &str, deep: bool, rate: f64) {
        if !rate.is_finite() || rate <= 0.0 {
            return;
        }
        // A zoomed view the core drew with the plain chaos game says nothing about deep views.
        let plain = |whole: &f64| rate * 3.0 > *whole;
        if deep && kind == FLAME && self.0.get(&(FLAME.to_owned(), false)).is_none_or(plain) {
            return;
        }
        let best = self.0.entry((kind.to_owned(), deep)).or_insert(rate);
        *best = best.max(rate);
    }

    fn rate(&self, part: &Part<'_>, deep: bool) -> Option<f64> {
        let of = |kind: &str, deep: bool| self.0.get(&(kind.to_owned(), deep)).copied();
        let whole = || {
            of(part.kind, false).or_else(|| {
                let flame = of(FLAME, false)?;
                Some(if part.stepped { flame * STEPS_A_SAMPLE } else { flame })
            })
        };
        match (deep, part.kind == FLAME) {
            (false, _) => whole(),
            (true, true) => of(FLAME, true).or_else(|| Some(whole()? / DEEP_COST)),
            (true, false) => of(part.kind, true).or_else(whole),
        }
    }

    // The seconds the processor is expected to take. Where nothing is known yet, a deep view of a flame is taken
    // for a long one and anything else for a short one.
    pub fn seconds(&self, parts: &[Part<'_>], deep: bool) -> f64 {
        parts
            .iter()
            .map(|part| match self.rate(part, deep) {
                Some(rate) => part.work / rate,
                None if deep && part.kind == FLAME => f64::INFINITY,
                None => 0.0,
            })
            .sum()
    }
}

// Whether a picture shows all of a view that is at [across, along] of it and `larger` times larger.
pub fn covers([across, along, larger]: [f32; 3]) -> bool {
    let half = 0.5 / larger;
    let inside = |at: f32| at - half >= -1e-3 && at + half <= 1.001;
    larger >= 0.999 && inside(across) && inside(along)
}

// The part of a picture around [across, along] of it, `larger` times larger, as a picture of another size.
pub fn magnified(rgba: &[u8], from: [u32; 2], [across, along, larger]: [f32; 3], to: [u32; 2]) -> Vec<u8> {
    let (wide, high) = (from[0] as usize, from[1] as usize);
    let mut out = Vec::with_capacity(to[0] as usize * to[1] as usize * 4);
    if wide == 0 || high == 0 || rgba.len() < wide * high * 4 {
        out.resize(to[0] as usize * to[1] as usize * 4, 0);
        return out;
    }
    let (across, along, larger) = (f64::from(across), f64::from(along), f64::from(larger).max(1e-9));
    let place = |at: u32, of: u32, middle: f64, side: usize| {
        let share = middle + ((f64::from(at) + 0.5) / f64::from(of) - 0.5) / larger;
        let pixel = (share * side as f64 - 0.5).clamp(0.0, (side - 1) as f64);
        let first = pixel.floor() as usize;
        (first, (first + 1).min(side - 1), pixel - pixel.floor())
    };
    let columns: Vec<_> = (0..to[0]).map(|x| place(x, to[0], across, wide)).collect();
    for y in 0..to[1] {
        let (upper, lower, down) = place(y, to[1], along, high);
        for &(left, right, along_row) in &columns {
            for channel in 0..4 {
                let at = |x: usize, y: usize| f64::from(rgba[(y * wide + x) * 4 + channel]);
                let above = at(left, upper) + (at(right, upper) - at(left, upper)) * along_row;
                let below = at(left, lower) + (at(right, lower) - at(left, lower)) * along_row;
                out.push((above + (below - above) * down).round() as u8);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_is_cut_to_its_slice_at_once_and_grows_slowly() {
        let slice = slice_of(false);
        assert!((slice - 0.5 / 60.0).abs() < 1e-6);
        // Four times too long: a quarter as large, at once.
        assert!((next(1000.0, 4.0 * slice, slice) - 250.0).abs() < 1e-3);
        // Half the slice: 0.8 of the slice at once.
        assert!((next(1000.0, 0.5 * slice, slice) - 1600.0).abs() < 1e-2);
        // Ten times too short: eight times, which is 0.8 of the slice.
        assert!((next(1000.0, 0.1 * slice, slice) - 8000.0).abs() < 1e-2);
        // A hundred times too short, as the first batch of an unknown card: eight times and no more.
        assert!((next(1000.0, 0.01 * slice, slice) - 8000.0).abs() < 1e-2);
        // Between 0.8 of the slice and the slice a batch stays as it is.
        assert!((next(1000.0, slice, slice) - 1000.0).abs() < 1e-3);
        assert!((next(1000.0, 0.9 * slice, slice) - 1000.0).abs() < 1e-3);
        // From the smallest batch a card a hundred thousand times faster is past 400 times it in three frames.
        let mut small = 1.0;
        for _ in 0..3 {
            small = next(small, (small * 1e-5 * f64::from(slice)) as f32, slice);
        }
        assert!(small > 400.0);
        // From any size a card ten times too slow is within its slice after one batch, and stays there.
        let mut sent = 1.0e6;
        let seconds = |sent: f64| (sent * 1e-7) as f32;
        sent = next(sent, seconds(sent), slice);
        for _ in 0..50 {
            assert!(seconds(sent) <= slice * 1.001);
            sent = next(sent, seconds(sent), slice);
        }
        assert!(seconds(sent) > slice * 0.79);
    }

    #[test]
    fn a_pointer_that_only_moves_does_not_use_the_window() {
        let at = egui::pos2(10.0, 10.0);
        let moved = [
            egui::Event::PointerMoved(at),
            egui::Event::MouseMoved(egui::vec2(1.0, 0.0)),
            egui::Event::PointerGone,
            egui::Event::WindowFocused(true),
        ];
        assert!(!used(false, &moved));
        assert!(!used(false, &[]));
        // A button held while it moves is a drag.
        assert!(used(true, &moved));
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            source: egui::MouseWheelSource::Unknown,
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        };
        let press = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        for event in [wheel, press, egui::Event::Zoom(1.1), egui::Event::Text("a".to_owned())] {
            assert!(used(false, &[egui::Event::PointerMoved(at), event]));
        }
    }

    #[test]
    fn a_draft_begins_at_a_quarter_a_side_with_its_margin() {
        let draft = Draft::new();
        assert_eq!(draft.side(2160), 4);
        assert_eq!(draft.size([3840, 2160]), [1200, 675]);
        // A small picture is not made smaller than a picture still is.
        assert_eq!(draft.side(520), 2);
        assert_eq!(draft.side(300), 2);
        assert_eq!(Draft::coarsest(2160), 5);
        assert_eq!(Draft::coarsest(1080), 3);
    }

    #[test]
    fn a_draft_that_overruns_is_made_smaller_at_once_and_finer_slowly() {
        let slice = slice_of(true);
        let mut draft = Draft::new();
        // Three times too long at a quarter: a sixth would take 4/9 of it and still overrun, an eighth fits.
        draft.took(3.0 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 8);
        // Just inside the slice: it stays.
        for _ in 0..100 {
            draft.took(0.9 * slice, slice, 2160);
        }
        assert_eq!(draft.side(2160), 8);
        // With room for the finer size it is made finer one step, after eight drafts and not before.
        for _ in 0..7 {
            draft.took(0.3 * slice, slice, 2160);
        }
        assert_eq!(draft.side(2160), 8);
        draft.took(0.3 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 6);
        // A draft without that room in between takes one from the count and does not start it again.
        for _ in 0..7 {
            draft.took(0.1 * slice, slice, 2160);
        }
        draft.took(0.95 * slice, slice, 2160);
        draft.took(0.1 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 6);
        draft.took(0.1 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 4);
        draft.took(1.5 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 6);
        // However slow the card, a draft is not made smaller than the floor, nor finer than half a side.
        draft.took(1000.0 * slice, slice, 2160);
        assert_eq!(draft.side(2160), 12);
        assert_eq!(draft.size([3840, 2160]), [400, 225]);
        draft.took(1000.0 * slice, slice, 1080);
        assert_eq!(draft.side(1080), 6);
        for _ in 0..1000 {
            draft.took(0.0001 * slice, slice, 2160);
        }
        assert_eq!(draft.side(2160), 2);
    }

    #[test]
    fn a_card_at_a_steady_speed_settles_on_one_size() {
        let slice = slice_of(true);
        // Seconds a pixel such that a quarter a side takes 1.6 slices.
        let pixels = |draft: &Draft| {
            let [wide, high] = draft.size([3840, 2160]);
            (wide * high) as f32
        };
        let each = 1.6 * slice / pixels(&Draft::new());
        let mut draft = Draft::new();
        let mut sides = Vec::new();
        for _ in 0..400 {
            draft.took(each * pixels(&draft), slice, 2160);
            sides.push(draft.side(2160));
        }
        assert!(sides[1..].iter().all(|side| *side == 6), "{sides:?}");
    }

    #[test]
    fn the_picture_follows_the_draft_of_the_very_view() {
        // A view that moved has no draft of its own on the screen: one is drawn.
        assert_eq!(hand_over(false, false), Next::Draft);
        // One draft at a time: while one is on its way nothing is begun, whatever the view does.
        assert_eq!(hand_over(true, false), Next::Wait);
        assert_eq!(hand_over(true, true), Next::Wait);
        // The draft on the screen is of this view, so the view stood still while it was drawn: the picture.
        assert_eq!(hand_over(false, true), Next::Picture);
    }

    #[test]
    fn what_is_beside_a_picture_is_the_share_it_does_not_cover() {
        assert_eq!(beside([0.5, 0.5, 1.0]), 0.0);
        assert_eq!(beside([0.5, 0.5, 4.0]), 0.0);
        assert!((beside([0.75, 0.5, 1.0]) - 0.25).abs() < 1e-6);
        assert!((beside([0.5, 0.5, 0.5]) - 0.75).abs() < 1e-6);
        // A draft with its margin covers a view that moved an eighth of itself since.
        let wider = 1.0 / DRAFT_MARGIN as f32;
        assert!(beside([0.5 + 0.124 * wider, 0.5, DRAFT_MARGIN as f32]) < 1e-6);
        assert!(beside([0.5 + 0.2 * wider, 0.5, DRAFT_MARGIN as f32]) > 0.05);
    }

    #[test]
    fn the_time_of_a_picture_counts_in_tenths_up_to_the_one_it_took() {
        assert_eq!(counted(0.0), None);
        assert_eq!(counted(0.09), None);
        assert_eq!(counted(0.1), Some(0.1));
        assert_eq!(counted(0.34), Some(0.3));
        let took = 2.2f32;
        let mut last = 0.0;
        for frame in 0..132 {
            let passed = frame as f32 / 60.0;
            if let Some(shown) = counted(passed) {
                assert!(shown >= last && shown <= took);
                last = shown;
            }
        }
        assert!(took - last < 0.11);
    }

    #[test]
    fn a_window_that_is_used_leaves_the_card_half_as_much() {
        assert!(slice_of(true) < slice_of(false));
        assert!((slice_of(true) - 0.25 / 60.0).abs() < 1e-6);
        attend(true);
        assert_eq!(slice(), slice_of(true));
        attend(false);
        assert_eq!(slice(), slice_of(false));
    }

    fn flame(work: f64) -> [Part<'static>; 1] {
        [Part {
            kind: FLAME,
            work,
            stepped: false,
        }]
    }

    #[test]
    fn a_deep_view_is_expected_to_take_ten_times_a_whole_one_until_one_is_measured() {
        let mut speeds = Speeds::default();
        assert_eq!(speeds.seconds(&flame(1e9), false), 0.0);
        assert_eq!(speeds.seconds(&flame(1e9), true), f64::INFINITY);
        speeds.seen(FLAME, false, 1e8);
        assert_eq!(speeds.seconds(&flame(1e9), false), 10.0);
        assert_eq!(speeds.seconds(&flame(1e9), true), 100.0);
        speeds.seen(FLAME, true, 2e7);
        assert_eq!(speeds.seconds(&flame(1e9), true), 50.0);
    }

    #[test]
    fn the_best_speed_counts_and_a_plain_render_of_a_zoomed_view_does_not() {
        let mut speeds = Speeds::default();
        speeds.seen(FLAME, false, 5e7);
        speeds.seen(FLAME, false, 1e8);
        speeds.seen(FLAME, false, 2e7);
        speeds.seen(FLAME, false, f64::NAN);
        assert_eq!(speeds.seconds(&flame(1e8), false), 1.0);
        speeds.seen(FLAME, true, 9e7);
        assert_eq!(speeds.seconds(&flame(1e8), true), 10.0);
    }

    #[test]
    fn a_kind_nothing_is_known_about_is_judged_by_the_flames() {
        let mut speeds = Speeds::default();
        let pixels = |kind| {
            [Part {
                kind,
                work: 6e9,
                stepped: true,
            }]
        };
        assert_eq!(speeds.seconds(&pixels("mandelbrot"), true), 0.0);
        speeds.seen(FLAME, false, 1e8);
        assert_eq!(speeds.seconds(&pixels("mandelbrot"), true), 1.0);
        speeds.seen("mandelbrot", false, 1e9);
        assert_eq!(speeds.seconds(&pixels("mandelbrot"), true), 6.0);
        assert_eq!(speeds.seconds(&pixels("julia"), false), 1.0);
        speeds.seen("mandelbrot", true, 2e8);
        assert_eq!(speeds.seconds(&pixels("mandelbrot"), true), 30.0);
        let both = [pixels("mandelbrot"), flame(1e9)].map(|[part]| part);
        assert_eq!(speeds.seconds(&both, false), 16.0);
    }

    #[test]
    fn a_picture_covers_the_views_inside_it() {
        assert!(covers([0.5, 0.5, 1.0]));
        assert!(covers([0.5, 0.5, 40.0]));
        assert!(covers([0.25, 0.75, 2.0]));
        assert!(!covers([0.2, 0.5, 2.0]));
        assert!(!covers([0.5, 0.5, 0.5]));
        assert!(!covers([1.4, 0.5, 100.0]));
    }

    #[test]
    fn a_magnified_picture_is_the_part_in_view() {
        let mut picture = Vec::new();
        for y in 0..4u8 {
            for x in 0..4u8 {
                picture.extend([x * 60, y * 60, 0, 255]);
            }
        }
        assert_eq!(magnified(&picture, [4, 4], [0.5, 0.5, 1.0], [4, 4]), picture);
        let corner = magnified(&picture, [4, 4], [0.75, 0.75, 2.0], [2, 2]);
        assert_eq!(
            corner,
            [120, 120, 0, 255, 180, 120, 0, 255, 120, 180, 0, 255, 180, 180, 0, 255]
        );
        let smooth = magnified(&picture, [4, 4], [0.5, 0.5, 4.0], [4, 1]);
        assert_eq!(smooth.len(), 16);
        assert!(smooth[0] > 60 && smooth[0] < smooth[4] && smooth[12] < 120);
    }
}
