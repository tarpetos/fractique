use eframe::egui;

pub const PANEL: f32 = 0.18;
pub const SMALL: f32 = 0.12;
// What a sheet covers gives way faster than the sheet comes, so the two are not read through each other.
pub const GIVE: f32 = 0.08;
pub const SHIFT: f32 = 12.0;
// The share of its blurred copy the picture is shown as while a new one is drawn.
pub const VEIL: f32 = 0.7;
// A picture that is ready sooner than this is never blurred.
pub const PATIENCE: f32 = 0.14;
pub const BAND: f32 = 24.0;
// The points of a window's width over which a panel makes room for what comes to stand under it.
pub const RAMP: f32 = 48.0;

// Smooth, not slow: nothing takes longer than a quarter of a second.
const _: () = assert!(PANEL <= 0.25 && SMALL <= PANEL && GIVE <= SMALL && PATIENCE <= 0.15);

pub fn ease(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - left * left * left
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

// The blur over the picture while a new one is drawn: it comes after a short patience, stays while the work
// runs and leaves when the picture is ready. Work that goes on keeps it, so a zoom in steps does not pulse.
#[derive(Clone, Copy, Debug)]
pub struct Veil {
    motion: Motion,
    busy_since: Option<f64>,
}

impl Veil {
    pub const fn new() -> Self {
        Self {
            motion: Motion::at(0.0),
            busy_since: None,
        }
    }

    // The strength now (0 to 1), and the seconds after which it has to be asked again although nothing moves.
    pub fn step(&mut self, busy: bool, now: f64) -> (f32, Option<f32>) {
        let mut wake = None;
        if busy {
            let since = *self.busy_since.get_or_insert(now);
            let waited = (now - since) as f32;
            if waited >= span(PATIENCE) || self.motion.value(now) > 0.0 {
                self.motion.aim(1.0, now, span(PANEL), 1.0);
            } else {
                wake = Some(span(PATIENCE) - waited);
            }
        } else {
            self.busy_since = None;
            self.motion.aim(0.0, now, span(PANEL), 1.0);
        }
        if self.motion.moving(now) {
            wake = Some(0.0);
        }
        (self.motion.value(now), wake)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn quick_work_never_blurs_and_long_work_blurs_once() {
        let mut veil = Veil::new();
        assert_eq!(veil.step(true, 0.0).0, 0.0);
        let (strength, wake) = veil.step(true, 0.1);
        assert_eq!(strength, 0.0);
        assert!(wake.is_some_and(|wake| wake > 0.0 && wake < PATIENCE));
        assert_eq!(veil.step(false, 0.12), (0.0, None));

        let mut veil = Veil::new();
        veil.step(true, 0.0);
        veil.step(true, 0.15);
        let rising = veil.step(true, 0.2).0;
        assert!(rising > 0.0 && rising < 1.0);
        assert_eq!(veil.step(true, 0.4).0, 1.0);
        // Work that goes on, as in a zoom step by step, keeps the blur where it is.
        assert_eq!(veil.step(true, 0.9), (1.0, None));
        let falling = veil.step(false, 1.0).0;
        assert_eq!(falling, 1.0);
        let lower = veil.step(false, 1.05).0;
        assert!(lower < 1.0 && lower > 0.0);
        // New work while it leaves turns it back from where it is, not from zero.
        let back = veil.step(true, 1.06).0;
        assert!(back > 0.0);
        assert!(veil.step(true, 1.3).0 == 1.0);
        assert_eq!(veil.step(false, 2.0).0, 1.0);
        assert_eq!(veil.step(false, 2.3), (0.0, None));
    }
}
