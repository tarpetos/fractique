use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

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

// A hand is on the window: a button or a finger held, or the wheel, a pinch or a key within the last 0.3 s. A
// press that is over is not, and neither is a pane in motion.
static HELD: AtomicBool = AtomicBool::new(false);

pub fn hold(held: bool) {
    HELD.store(held, Ordering::Relaxed);
}

pub fn held() -> bool {
    HELD.load(Ordering::Relaxed)
}

// What the card needs for the window's own frame, in microseconds: the most of late, which fades when nothing
// comes near it again. Nothing until a frame has told.
static NEED: AtomicU32 = AtomicU32::new(0);
// The microseconds the card took for the window's last frame, from its submission to its end, until it is taken.
pub static FRAME_ON_CARD: AtomicU32 = AtomicU32::new(0);
// A test run's fixed share of a frame, in thousandths; nothing where the share goes by what the window needs.
static FIXED: AtomicU32 = AtomicU32::new(0);
// What stays free of a frame beside the window's own work and the picture's: a unit of the picture's work that
// began at the last moment, the way back from it, and room for a frame that is dearer than the last.
pub const MARGIN: f32 = 0.003;
// The most of a frame a picture gets, however little the window needs, and the least, however much.
pub const MOST_SHARE: f32 = 0.8;
pub const LEAST: f32 = 0.002;

// A frame that took the card longer than a frame, as the first run of a shader does, counts as a whole frame and
// is forgotten within a few: what the window needs in a fade over a large picture, 7 to 9 ms, within half a second.
pub fn needs(micros: u32) {
    let before = NEED.load(Ordering::Relaxed);
    let most = (FRAME * 1e6) as u32;
    NEED.store(micros.min(most).max(before - before / 16), Ordering::Relaxed);
}

pub fn fix(share: f32) {
    FIXED.store((share.clamp(0.05, 0.9) * 1000.0) as u32, Ordering::Relaxed);
}

// The seconds of a frame the card may work on a picture: the frame, less what the card needs for the window's own
// frame, less the margin; half of that while the window is used. Half a frame where the window's need is not
// known yet.
pub fn allowance(need: Option<f32>, used: bool) -> f32 {
    let idle = match need {
        Some(need) => (FRAME - need - MARGIN).clamp(LEAST, MOST_SHARE * FRAME),
        None => SHARE * FRAME,
    };
    if used { 0.5 * idle } else { idle }
}

// The seconds of this frame the units of a picture may be sent in: one follows the other until then.
pub fn slice() -> f32 {
    let used = ATTENDED.load(Ordering::Relaxed);
    match (FIXED.load(Ordering::Relaxed), NEED.load(Ordering::Relaxed)) {
        (0, 0) => allowance(None, used),
        (0, need) => allowance(Some(need as f32 * 1e-6), used),
        (share, _) => allowance(Some(FRAME * (1.0 - share as f32 * 1e-3) - MARGIN), used),
    }
}

// The seconds one batch of a picture drawn pixel by pixel or of a wallpaper is sized for: their batches follow
// one another without a slice of the frame.
pub fn batch() -> f32 {
    slice_of(ATTENDED.load(Ordering::Relaxed))
}

pub fn slice_of(attended: bool) -> f32 {
    FRAME * if attended { SHARE_ATTENDED } else { SHARE }
}

// Whether a unit of the card's work may begin, so many seconds after the frame opened its slice. A unit that has
// begun runs to its end: the frame is late by one unit at the very most, never by a slice.
pub fn within(spent: f32, slice: f32) -> bool {
    spent < slice
}

// The time shown while a picture is on its way: the seconds since it was asked for, a tenth at a step, and
// nothing before the first tenth. It never passes the time the finished picture is shown with.
pub fn counted(seconds: f32) -> Option<f32> {
    (seconds >= 0.1).then(|| (seconds * 10.0).floor() / 10.0)
}

pub fn let_go(at: egui::Pos2) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }
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
    held || events.iter().any(|event| !idle(event))
}

fn idle(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::PointerMoved(_)
            | egui::Event::MouseMoved(_)
            | egui::Event::PointerGone
            | egui::Event::WindowFocused(_)
    )
}

// Whether what the window was told in a frame is a hand that goes on: the wheel, a pinch, a key, a text. A press
// or its release is not: a tap is over when the picture it asked for begins, and a finger that stays down is
// seen as held.
pub fn turned(events: &[egui::Event]) -> bool {
    let press = |event: &egui::Event| matches!(event, egui::Event::PointerButton { .. } | egui::Event::Touch { .. });
    events.iter().any(|event| !idle(event) && !press(event))
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
    (1.0 - covered(across) * covered(along)).max(0.0)
}

// Where a view at [across, along] of a picture's own view, `larger` times larger than it, is in the picture when
// that is drawn so many times wider and higher than its view, with a margin around it.
pub fn widened([across, along, larger]: [f32; 3], [wider, higher]: [f64; 2]) -> [f32; 3] {
    let (wider, higher) = (wider as f32, higher as f32);
    [
        0.5 + (across - 0.5) / wider,
        0.5 + (along - 0.5) / higher,
        larger * wider.min(higher),
    ]
}

// The margins a picture made of samples may be drawn with around its view, as shares of the view's sides on every
// side, the widest first: a view that moves, or shows more of the scene, inside the margin has its picture at once.
pub const MARGINS: [f64; 4] = [0.5, 0.375, 0.25, 0.125];
// A deep view's windows halve down to its view: the picture with a margin is one of them or between two.
const _: () = assert!(MARGINS[0] <= 0.5);
// The picture with a margin is drawn behind the finished one, when nothing is waited for. It is drawn where it is
// expected within this many seconds: later than that the view has moved on, and the card has worked for nothing.
pub const MARGIN_WITHIN: f64 = 3.0;

// The margins worth drawing, the widest first, around a picture whose view alone took the card so many seconds.
// The samples of a picture with a margin are those of its view, and its cells are more by the square of its
// width: on an RTX 4070 it took 1 to 1.5 times the seconds of the view alone at a quarter and at a half on every
// side, and is taken to cost as many times the seconds as it is wider. A card nothing is known about yet, or a
// slow one, draws none.
pub fn margins(took: Option<f32>) -> impl Iterator<Item = f64> {
    MARGINS.into_iter().filter(move |share| {
        let wider = 1.0 + 2.0 * share;
        took.is_some_and(|took| f64::from(took) * wider <= MARGIN_WITHIN)
    })
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

// A picture made smaller by averaging: every pixel of the new one is the mean of the pixels of the old one it
// covers, each by the share of it that is covered. At a whole ratio that is the mean of a block.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn shrunk(rgba: &[u8], from: [u32; 2], to: [u32; 2]) -> Vec<u8> {
    let (wide, high) = (from[0] as usize, from[1] as usize);
    let (across, down) = (to[0] as usize, to[1] as usize);
    if wide == 0 || high == 0 || rgba.len() < wide * high * 4 {
        return vec![0; across * down * 4];
    }
    // For each pixel of the new side: the first old pixel it covers, and the share of each it covers.
    let spans = |old: usize, new: usize| -> Vec<(usize, Vec<f32>)> {
        let ratio = old as f64 / new.max(1) as f64;
        (0..new)
            .map(|at| {
                let (begin, end) = (at as f64 * ratio, ((at + 1) as f64 * ratio).min(old as f64));
                let first = (begin.floor() as usize).min(old - 1);
                let last = ((end.ceil() as usize).max(first + 1)).min(old);
                let shares = (first..last)
                    .map(|pixel| {
                        let covered = (end.min((pixel + 1) as f64) - begin.max(pixel as f64)).max(0.0);
                        (covered / (end - begin).max(1e-12)) as f32
                    })
                    .collect();
                (first, shares)
            })
            .collect()
    };
    let (columns, rows) = (spans(wide, across), spans(high, down));
    let mut narrow = vec![0f32; across * high * 4];
    for (y, out) in narrow.chunks_exact_mut(across * 4).enumerate() {
        let row = &rgba[y * wide * 4..(y + 1) * wide * 4];
        for ((first, shares), pixel) in columns.iter().zip(out.chunks_exact_mut(4)) {
            for (offset, share) in shares.iter().enumerate() {
                let old = &row[(first + offset) * 4..(first + offset) * 4 + 4];
                for channel in 0..4 {
                    pixel[channel] += f32::from(old[channel]) * share;
                }
            }
        }
    }
    let mut out = Vec::with_capacity(across * down * 4);
    let mut sums = vec![0f32; across * 4];
    for (first, shares) in &rows {
        sums.fill(0.0);
        for (offset, share) in shares.iter().enumerate() {
            let row = &narrow[(first + offset) * across * 4..(first + offset + 1) * across * 4];
            for (sum, value) in sums.iter_mut().zip(row) {
                *sum += value * share;
            }
        }
        out.extend(sums.iter().map(|sum| (sum + 0.5).clamp(0.0, 255.0) as u8));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_made_smaller_is_the_mean_of_the_pixels_it_covers() {
        let (wide, high) = (12usize, 8usize);
        let rgba: Vec<u8> = (0..wide * high * 4)
            .map(|at| ((at * 37 + at / 7) % 251) as u8)
            .collect();
        let at = |x: usize, y: usize, channel: usize| f64::from(rgba[(y * wide + x) * 4 + channel]);
        // Whole ratios: the mean of a block, exactly.
        for by in [1usize, 2, 4] {
            let to = [wide / by, high / by];
            let small = shrunk(&rgba, [wide as u32, high as u32], to.map(|side| side as u32));
            assert_eq!(small.len(), to[0] * to[1] * 4);
            for y in 0..to[1] {
                for x in 0..to[0] {
                    for channel in 0..4 {
                        let block = (0..by).flat_map(|dy| (0..by).map(move |dx| (dx, dy)));
                        let sum: f64 = block.map(|(dx, dy)| at(x * by + dx, y * by + dy, channel)).sum();
                        let mean = (sum / (by * by) as f64 + 0.5).floor();
                        let got = f64::from(small[(y * to[0] + x) * 4 + channel]);
                        assert!((got - mean).abs() <= 1.0, "{by}: {got} for {mean}");
                    }
                }
            }
        }
        assert_eq!(shrunk(&rgba, [12, 8], [12, 8]), rgba);
        // Another ratio: each old pixel by the share of it that is covered. 3 pixels to 2: 1 and a half each.
        let row = [30u8, 0, 0, 255, 90, 0, 0, 255, 150, 0, 0, 255];
        assert_eq!(shrunk(&row, [3, 1], [2, 1]), [50, 0, 0, 255, 130, 0, 0, 255]);
        // What a flat picture is made of stays what it is.
        assert!(
            shrunk(&[200; 7 * 5 * 4], [7, 5], [3, 2])
                .iter()
                .all(|value| *value == 200)
        );
    }

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
    fn a_unit_never_begins_past_the_slice_and_a_frame_is_late_by_one_at_most() {
        // Units of 0.4 to 2.1 ms, as a card gives them, sent one after the other in slices of every length.
        let lengths = [0.0011f32, 0.0004, 0.0021, 0.0009, 0.0013, 0.0007];
        for slice in [slice_of(true), slice_of(false), 0.0115, 0.0005] {
            let (mut at, mut sent, mut next) = (0.0f32, 0, 0);
            while within(at, slice) {
                at += lengths[next % lengths.len()];
                next += 1;
                sent += 1;
            }
            // The last one began inside the slice and ended within one unit of its end.
            assert!(sent >= 1 && at - slice <= 0.0021 + 1e-6, "{slice}: {at}");
            // The slice was filled but for less than the shortest unit.
            assert!(at >= slice);
            assert!(!within(at, slice));
        }
        // Nothing begins in a slice that is over.
        assert!(!within(0.0042, 0.0042));
        assert!(within(0.0041, 0.0042));
    }

    #[test]
    fn the_slice_is_what_the_window_leaves_and_half_of_it_while_the_window_is_used() {
        // A window whose frame costs the card 1 ms leaves 12.7 ms of 16.7; one that costs 6 ms, 7.7.
        assert!((allowance(Some(0.001), false) - (FRAME - 0.004)).abs() < 1e-6);
        assert!((allowance(Some(0.006), false) - (FRAME - 0.009)).abs() < 1e-6);
        // Never more than 0.8 of a frame, and never nothing.
        assert!((allowance(Some(0.0), false) - 0.8 * FRAME).abs() < 1e-6);
        assert_eq!(allowance(Some(0.016), false), LEAST);
        // While the window is used the picture has half of that, at once: a unit that has not begun does not
        // begin past the new slice.
        let idle = allowance(Some(0.001), false);
        assert!((allowance(Some(0.001), true) - 0.5 * idle).abs() < 1e-6);
        assert!(within(0.008, idle) && !within(0.008, allowance(Some(0.001), true)));
        // A device that has not told what its window needs: half a frame, a quarter while used.
        assert_eq!(allowance(None, false), slice_of(false));
        assert_eq!(allowance(None, true), slice_of(true));
        // What the window needs is the most of late, and fades when nothing comes near it again.
        NEED.store(0, Ordering::Relaxed);
        needs(2000);
        needs(500);
        assert!(NEED.load(Ordering::Relaxed) > 1800);
        // One frame that stalled is a frame at the most, and gone from the slice within ten.
        needs(45_000);
        assert_eq!(NEED.load(Ordering::Relaxed), 16_666);
        for _ in 0..10 {
            needs(500);
        }
        assert!(allowance(Some(NEED.load(Ordering::Relaxed) as f32 * 1e-6), false) > 0.004);
        for _ in 0..400 {
            needs(500);
        }
        assert_eq!(NEED.load(Ordering::Relaxed), 500);
        NEED.store(0, Ordering::Relaxed);
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
        assert_eq!(batch(), slice_of(true));
        attend(false);
        assert_eq!(batch(), slice_of(false));
    }

    #[test]
    fn a_tap_that_is_over_is_no_hand_on_the_window() {
        let button = |pressed| egui::Event::PointerButton {
            pos: egui::Pos2::ZERO,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(used(false, &[button(false)]));
        assert!(!turned(&[
            button(true),
            button(false),
            egui::Event::PointerMoved(egui::Pos2::ZERO)
        ]));
        assert!(turned(&[egui::Event::Zoom(1.1)]));
        assert!(turned(&[egui::Event::Text("a".to_owned())]));
        hold(true);
        assert!(held());
        hold(false);
        assert!(!held());
    }

    #[test]
    fn a_button_the_system_took_with_the_window_is_let_go() {
        let ctx = egui::Context::default();
        let at = egui::pos2(40.0, 20.0);
        let frame = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |_| {});
            ctx.input(|input| input.pointer.any_down())
        };
        let press = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(frame(vec![egui::Event::PointerMoved(at), press]));
        assert!(frame(vec![egui::Event::PointerGone]));
        assert!(!frame(vec![let_go(at)]));
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
    fn a_picture_with_a_margin_holds_the_views_that_moved_inside_it() {
        // A quarter of the view on every side: one and a half times as wide and as high.
        let wide = [1.5, 1.5];
        assert_eq!(widened([0.5, 0.5, 1.0], wide), [0.5, 0.5, 1.5]);
        // The view moved a quarter of itself sideways: it ends at the picture's edge, and is all there.
        assert!(covers(widened([0.75, 0.5, 1.0], wide)));
        assert!(covers(widened([0.25, 0.25, 1.0], wide)));
        assert!(!covers(widened([0.8, 0.5, 1.0], wide)));
        assert!(beside(widened([0.75, 0.5, 1.0], wide)) < 1e-6);
        // Past the margin, the share beside the picture is what the view moved past it by.
        assert!((beside(widened([1.0, 0.5, 1.0], wide)) - 0.25).abs() < 1e-5);
        assert!((beside([1.0, 0.5, 1.0]) - 0.5).abs() < 1e-6);
        // A view that shows one and a half times as much of the scene, around the same point, is all there too;
        // one that shows twice as much is not.
        assert!(covers(widened([0.5, 0.5, 1.0 / 1.5], wide)));
        assert!(!covers(widened([0.5, 0.5, 0.5], wide)));
        assert!((beside(widened([0.5, 0.5, 0.5], wide)) - (1.0 - 0.75 * 0.75)).abs() < 1e-5);
        // Smaller and moved: what it gained across it may not move by.
        assert!(covers(widened([0.55, 0.5, 0.8], wide)));
        assert!(!covers(widened([0.7, 0.5, 0.8], wide)));
        // Without a margin nothing changes.
        assert_eq!(widened([0.3, 0.6, 2.0], [1.0, 1.0]), [0.3, 0.6, 2.0]);
    }

    #[test]
    fn a_margin_is_as_wide_as_the_card_draws_in_time() {
        let all = |took| margins(took).collect::<Vec<_>>();
        // Nothing is known of the card yet: no margin.
        assert!(all(None).is_empty());
        // A view the card draws in 0.3 s: with half of it on every side it is taken to cost 0.6 s.
        assert_eq!(all(Some(0.3)), MARGINS);
        // 1.4 s, a deep view of 3840 × 2160 on an RTX 4070: every margin still, 2.8 s at the widest.
        assert_eq!(all(Some(1.4)), MARGINS);
        // Two seconds: a quarter on every side would be 3 s, and is the widest. A slow card draws a narrow margin
        // or none, by the same rule.
        assert_eq!(all(Some(2.0)), [0.25, 0.125]);
        assert_eq!(all(Some(2.3)), [0.125]);
        assert!(all(Some(2.5)).is_empty());
        assert!(all(Some(8.0)).is_empty());
        // The widest comes first, and none is wider than the view is.
        assert!(MARGINS.windows(2).all(|pair| pair[0] > pair[1]));
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
