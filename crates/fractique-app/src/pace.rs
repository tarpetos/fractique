use std::collections::HashMap;

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
