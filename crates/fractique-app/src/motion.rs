use eframe::egui;

pub const PANEL: f32 = 0.18;
pub const SMALL: f32 = 0.12;
// What a sheet covers gives way faster than the sheet comes, so the two are not read through each other.
pub const GIVE: f32 = 0.08;
pub const SHIFT: f32 = 12.0;
// The share of its blurred copy the picture is shown as while a new one is drawn.
pub const VEIL: f32 = 0.7;
// The blur is on its way in the frame that asks for a picture: it starts one frame into its rise.
const FRAME: f64 = 1.0 / 60.0;
pub const BAND: f32 = 24.0;
// The points of a window's width over which a panel makes room for what comes to stand under it.
pub const RAMP: f32 = 48.0;

// Smooth, not slow: nothing takes longer than a quarter of a second.
const _: () = assert!(PANEL <= 0.25 && SMALL <= PANEL && GIVE <= SMALL);

pub fn ease(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - left * left * left
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// False where the system asks for no animations: every span is then zero. FRACTIQUE_MOTION=off does the same.
pub fn wanted() -> bool {
    static WANTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *WANTED.get_or_init(|| std::env::var("FRACTIQUE_MOTION").map_or_else(|_| system_wants(), |choice| choice != "off"))
}

#[cfg(windows)]
fn system_wants() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW};
    let mut on: i32 = 1;
    // SAFETY: the call writes one BOOL through the pointer, which is valid for the call.
    let asked = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut on).cast(), 0) };
    asked == 0 || on != 0
}

#[cfg(not(windows))]
fn system_wants() -> bool {
    true
}

pub fn span(seconds: f32) -> f32 {
    if wanted() { seconds } else { 0.0 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    from: f32,
    to: f32,
    since: f64,
    span: f32,
}

impl Motion {
    pub const fn at(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            since: 0.0,
            span: 0.0,
        }
    }

    // A new aim starts from where the value is, and takes the share of the span that is left to travel.
    pub fn aim(&mut self, to: f32, now: f64, span: f32, reach: f32) {
        if to == self.to {
            return;
        }
        let from = self.value(now);
        let share = ((to - from).abs() / reach.max(f32::EPSILON)).min(1.0);
        *self = Self {
            from,
            to,
            since: now,
            span: span * share,
        };
    }

    pub fn value(&self, now: f64) -> f32 {
        if self.span <= 0.0 {
            return self.to;
        }
        let t = ((now - self.since) as f32 / self.span).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease(t)
    }

    pub fn moving(&self, now: f64) -> bool {
        self.span > 0.0 && ((now - self.since) as f32) < self.span
    }

    pub fn target(&self) -> f32 {
        self.to
    }
}

// What is on the screen and how far each part is shown: 0 hidden, 1 whole.
#[derive(Default)]
pub struct Stage {
    parts: Vec<(&'static str, Motion)>,
    pub now: f64,
}

impl Stage {
    pub fn begin(&mut self, now: f64) {
        self.now = now;
    }

    fn part(&mut self, key: &'static str, first: f32) -> &mut Motion {
        let at = match self.parts.iter().position(|(name, _)| *name == key) {
            Some(at) => at,
            None => {
                self.parts.push((key, Motion::at(first)));
                self.parts.len() - 1
            }
        };
        &mut self.parts[at].1
    }

    // How far a part is shown, on its way to open or closed; None once it is closed and need not be drawn.
    pub fn shown(&mut self, key: &'static str, open: bool, seconds: f32) -> Option<f32> {
        let now = self.now;
        let part = self.part(key, 0.0);
        part.aim(if open { 1.0 } else { 0.0 }, now, span(seconds), 1.0);
        let value = part.value(now);
        (open || value > 0.0).then_some(value)
    }

    // A length that glides to a new value instead of jumping there; the first value is taken as it is.
    pub fn glide(&mut self, key: &'static str, to: f32, seconds: f32) -> f32 {
        let now = self.now;
        let part = self.part(key, to);
        let reach = (to - part.value(now)).abs();
        part.aim(to, now, span(seconds), reach);
        part.value(now)
    }

    pub fn peek(&self, key: &'static str) -> Option<f32> {
        self.parts
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, part)| part.value(self.now))
    }

    pub fn aim_of(&self, key: &'static str) -> Option<f32> {
        self.parts
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, part)| part.target())
    }

    pub fn moving(&self) -> bool {
        self.parts.iter().any(|(_, part)| part.moving(self.now))
    }

    pub fn keep_moving(&self, ctx: &egui::Context) {
        if self.moving() {
            ctx.request_repaint();
        }
    }
}

// A switch at a threshold that does not flap around it: it turns on above the band and off below it.
pub fn latch(on: Option<bool>, value: f32, threshold: f32, band: f32) -> bool {
    match on {
        None => value >= threshold,
        Some(true) => value >= threshold - band,
        Some(false) => value >= threshold + band,
    }
}

// A value that has to stand still for a while before it counts: a window's size during a drag of its edge.
#[derive(Clone, Copy, Debug)]
pub struct Rest<T> {
    seen: Option<(T, f64)>,
}

impl<T: PartialEq + Copy> Rest<T> {
    pub const fn new() -> Self {
        Self { seen: None }
    }

    // The seconds the value still has to stand; zero when it has rested, and at once for the first value.
    pub fn wait(&mut self, value: T, now: f64, seconds: f32) -> f32 {
        match self.seen {
            None => {
                self.seen = Some((value, now - f64::from(seconds)));
                0.0
            }
            Some((seen, _)) if seen != value => {
                self.seen = Some((value, now));
                seconds
            }
            Some((_, since)) => (seconds - (now - since) as f32).max(0.0),
        }
    }
}

// The blur over the picture while a new one is on its way. It begins in the frame that asks for the picture and
// rises fast (`GIVE`); it stays while the picture is awaited, so steps that follow one another keep it; it
// leaves (`PANEL`) once the picture is there, and never before it has fully come, so the shortest blur is one
// rise and one fall and none flicks. A picture that is there in the frame that asked for it gets no blur. While
// a button or a finger is held, as on a slider that is dragged, it does not leave between two pictures.
#[derive(Clone, Copy, Debug)]
pub struct Veil {
    from: f32,
    to: f32,
    since: f64,
    span: f32,
    began: f64,
}

impl Veil {
    pub const fn new() -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            since: 0.0,
            span: 0.0,
            began: 0.0,
        }
    }

    fn value(&self, now: f64) -> f32 {
        if self.span <= 0.0 {
            return self.to;
        }
        let t = (now - self.since) as f32 / self.span;
        let share = if self.to > self.from { ease(t) } else { smooth(t) };
        self.from + (self.to - self.from) * share
    }

    fn turning(&self, now: f64) -> bool {
        self.span > 0.0 && ((now - self.since) as f32) < self.span
    }

    // The strength now (0 to 1), and whether the next frame has to be drawn for it.
    pub fn step(&mut self, awaited: bool, held: bool, now: f64) -> (f32, bool) {
        if !wanted() {
            return (0.0, false);
        }
        let value = self.value(now);
        if awaited && self.to != 1.0 {
            let fresh = value <= 0.0;
            if fresh {
                self.began = now;
            }
            *self = Self {
                from: value,
                to: 1.0,
                since: if fresh { now - FRAME } else { now },
                span: GIVE * (1.0 - value),
                began: self.began,
            };
        } else if !awaited && !held && self.to != 0.0 && !self.turning(now) {
            *self = Self {
                from: value,
                to: 0.0,
                since: now,
                span: PANEL * value,
                began: self.began,
            };
        }
        (
            self.value(now),
            self.turning(now) || (!awaited && !held && self.to != 0.0),
        )
    }
}

// What the screen takes of the pictures offered to it. While a picture is awaited the screen shows two things
// only: one still picture under the whole blur, then the finished one. A first, small picture of the new one may
// take the place of the old one once, under the whole blur; nothing else changes until the finished picture
// fades in as the blur leaves. Without a blur, the finished picture is taken as it comes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn {
    Stay,
    First,
    Last,
}

pub fn turn(veil: f32, awaited: bool, fading: bool, first: bool, last: bool) -> Turn {
    if fading {
        Turn::Stay
    } else if veil <= 0.0 && !awaited {
        if last { Turn::Last } else { Turn::Stay }
    } else if veil < 1.0 {
        Turn::Stay
    } else if last {
        Turn::Last
    } else if first {
        Turn::First
    } else {
        Turn::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_the_blur_the_screen_changes_once_at_most_before_the_finished_picture() {
        // Nothing is taken while the blur comes, nor while one picture fades into another.
        assert_eq!(turn(0.5, true, false, true, true), Turn::Stay);
        assert_eq!(turn(1.0, true, true, true, true), Turn::Stay);
        // Under the whole blur the first picture is taken, and the finished one before it when both are there.
        assert_eq!(turn(1.0, true, false, true, false), Turn::First);
        assert_eq!(turn(1.0, true, false, true, true), Turn::Last);
        assert_eq!(turn(1.0, false, false, false, true), Turn::Last);
        assert_eq!(turn(1.0, true, false, false, false), Turn::Stay);
        // The blur leaving is no time for a first picture.
        assert_eq!(turn(0.4, false, false, true, false), Turn::Stay);
    }

    #[test]
    fn without_a_blur_only_the_finished_picture_is_shown() {
        assert_eq!(turn(0.0, false, false, true, false), Turn::Stay);
        assert_eq!(turn(0.0, false, false, true, true), Turn::Last);
        assert_eq!(turn(0.0, false, true, false, true), Turn::Stay);
        // Asked for in this frame: the blur is about to come, and the picture waits for it.
        assert_eq!(turn(0.0, true, false, false, true), Turn::Stay);
    }

    #[test]
    fn easing_starts_fast_ends_at_rest_and_never_goes_back() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(7.0), 1.0);
        assert!(ease(0.5) > 0.5);
        let mut last = 0.0;
        for step in 0..=100 {
            let now = ease(step as f32 / 100.0);
            assert!(now >= last);
            last = now;
        }
        assert!(ease(1.0) - ease(0.9) < ease(0.1) - ease(0.0));
    }

    #[test]
    fn a_motion_reaches_its_aim_in_its_span_without_a_jump() {
        let mut motion = Motion::at(0.0);
        motion.aim(1.0, 10.0, 0.2, 1.0);
        assert_eq!(motion.value(10.0), 0.0);
        assert!(motion.moving(10.1));
        assert_eq!(motion.value(10.2), 1.0);
        assert!(!motion.moving(10.2));
        let frame = 1.0 / 60.0;
        let mut last = 0.0;
        for step in 0..=12 {
            let value = motion.value(10.0 + f64::from(step) * frame);
            assert!(value - last <= ease(frame as f32 / 0.2) + 1e-4);
            last = value;
        }
    }

    #[test]
    fn a_motion_turned_back_midway_goes_on_from_where_it_is() {
        let mut motion = Motion::at(0.0);
        motion.aim(1.0, 0.0, 0.2, 1.0);
        let reached = motion.value(0.05);
        assert!(reached > 0.0 && reached < 1.0);
        motion.aim(0.0, 0.05, 0.2, 1.0);
        assert!((motion.value(0.05) - reached).abs() < 1e-6);
        assert!(motion.value(0.06) < reached);
        // Back from part of the way takes that part of the span.
        assert_eq!(motion.value(0.05 + 0.2 * f64::from(reached) + 1e-6), 0.0);
        motion.aim(0.0, 0.07, 0.2, 1.0);
        assert!(motion.moving(0.07));
    }

    #[test]
    fn a_part_of_the_stage_opens_closes_and_is_gone_only_when_closed() {
        let mut stage = Stage::default();
        stage.begin(0.0);
        assert_eq!(stage.shown("list", false, 0.2), None);
        assert_eq!(stage.shown("list", true, 0.2), Some(0.0));
        stage.begin(0.1);
        let half = stage.shown("list", true, 0.2).unwrap();
        assert!(half > 0.5 && half < 1.0 && stage.moving());
        assert_eq!(stage.shown("list", false, 0.2), Some(half));
        stage.begin(0.4);
        assert_eq!(stage.shown("list", false, 0.2), None);
        assert!(!stage.moving());
    }

    #[test]
    fn a_length_glides_from_its_first_value() {
        let mut stage = Stage::default();
        stage.begin(1.0);
        assert_eq!(stage.glide("under", 72.0, 0.2), 72.0);
        assert_eq!(stage.glide("under", 0.0, 0.2), 72.0);
        stage.begin(1.1);
        let midway = stage.glide("under", 0.0, 0.2);
        assert!(midway > 0.0 && midway < 36.0);
        stage.begin(1.2);
        assert_eq!(stage.glide("under", 0.0, 0.2), 0.0);
    }

    #[test]
    fn a_latch_does_not_flap_around_its_threshold() {
        assert!(latch(None, 700.0, 700.0, 24.0));
        assert!(!latch(None, 699.0, 700.0, 24.0));
        let mut on = true;
        let mut flips = 0;
        for width in [705.0, 695.0, 702.0, 690.0, 710.0, 677.0, 723.0] {
            let next = latch(Some(on), width, 700.0, 24.0);
            flips += u32::from(next != on);
            on = next;
        }
        assert_eq!(flips, 0);
        assert!(!latch(Some(true), 675.0, 700.0, 24.0));
        assert!(latch(Some(false), 724.0, 700.0, 24.0));
    }

    #[test]
    fn a_size_counts_once_it_has_stood_still() {
        let mut rest = Rest::new();
        assert_eq!(rest.wait([800, 600], 5.0, 0.18), 0.0);
        assert_eq!(rest.wait([790, 600], 5.01, 0.18), 0.18);
        assert_eq!(rest.wait([780, 600], 5.02, 0.18), 0.18);
        assert!((rest.wait([780, 600], 5.1, 0.18) - 0.1).abs() < 1e-4);
        assert_eq!(rest.wait([780, 600], 5.21, 0.18), 0.0);
    }

    fn course(veil: &mut Veil, awaited: impl Fn(u32) -> (bool, bool), frames: u32) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                let (awaited, held) = awaited(frame);
                veil.step(awaited, held, f64::from(frame) / 60.0).0
            })
            .collect()
    }

    fn humps(course: &[f32]) -> u32 {
        let mut count = 0;
        let mut falling = false;
        for pair in course.windows(2) {
            if pair[1] < pair[0] {
                falling = true;
            } else if pair[1] > pair[0] && falling {
                count += 1;
                falling = false;
            }
        }
        count + u32::from(course.iter().any(|value| *value > 0.0))
    }

    #[test]
    fn the_blur_is_on_its_way_in_the_frame_that_asks_for_a_picture() {
        let mut veil = Veil::new();
        assert_eq!(veil.step(false, false, 0.0), (0.0, false));
        let (first, moving) = veil.step(true, false, 1.0);
        assert!(first > 0.3 && first < 1.0 && moving);
        assert_eq!(veil.step(true, false, 1.0 + f64::from(GIVE)).0, 1.0);
    }

    #[test]
    fn a_picture_that_is_there_at_once_is_never_blurred() {
        let mut veil = Veil::new();
        assert!(
            course(&mut veil, |_| (false, false), 30)
                .iter()
                .all(|value| *value == 0.0)
        );
    }

    #[test]
    fn the_shortest_blur_comes_fully_and_leaves_once() {
        let mut veil = Veil::new();
        // The picture is awaited for a single frame.
        let course = course(&mut veil, |frame| (frame == 3, false), 40);
        assert_eq!(course[2], 0.0);
        assert!(course[3] > 0.0);
        let peak = course
            .iter()
            .position(|value| *value == 1.0)
            .expect("the blur comes fully");
        assert!(course[..=peak].windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(course[peak..].windows(2).all(|pair| pair[1] <= pair[0]));
        assert_eq!(humps(&course), 1);
        let shown = course.iter().filter(|value| **value > 0.0).count();
        assert!(shown as f32 / 60.0 >= 0.2, "{shown} frames");
        assert_eq!(*course.last().unwrap(), 0.0);
        assert_eq!(veil.step(false, false, 2.0), (0.0, false));
    }

    #[test]
    fn steps_that_follow_one_another_keep_the_blur_and_it_leaves_once() {
        let mut veil = Veil::new();
        // A slider dragged for a second: each of its pictures is there two frames after it is asked for.
        let dragged = |frame: u32| {
            (
                (5..70).contains(&frame) && !frame.is_multiple_of(3),
                (5..70).contains(&frame),
            )
        };
        let course = course(&mut veil, dragged, 100);
        assert_eq!(humps(&course), 1);
        assert!(course[12..70].iter().all(|value| *value == 1.0));
        assert!(course[71] < 1.0);
        assert_eq!(*course.last().unwrap(), 0.0);
        // Held at its level, it asks for no frames.
        let mut held = Veil::new();
        held.step(true, false, 0.0);
        assert_eq!(held.step(true, false, 1.0), (1.0, false));
        assert_eq!(held.step(false, true, 2.0), (1.0, false));
    }

    #[test]
    fn a_picture_asked_for_while_the_blur_leaves_turns_it_back_without_a_jump() {
        let mut veil = Veil::new();
        veil.step(true, false, 0.0);
        veil.step(false, false, 0.2);
        let leaving = veil.step(false, false, 0.3).0;
        assert!(leaving > 0.0 && leaving < 1.0);
        let back = veil.step(true, false, 0.3).0;
        assert!((back - leaving).abs() < 1e-6);
        assert_eq!(veil.step(true, false, 0.4).0, 1.0);
    }
}
