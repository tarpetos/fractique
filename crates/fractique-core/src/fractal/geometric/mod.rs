use std::ops::{Add, Div, Mul, Neg, Sub};

use super::{Orbit, Param, Walk};
use crate::math::exp;
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};

super::family! {
    apollonian,
    kleinian,
    l_system,
    lyapunov,
}

const COLOR_DENSITY: Param = Param::real("color_density", 1.0, 0.01, 100.0).log();
const COLOR_SHIFT: Param = Param::real("color_shift", 0.0, 0.0, 2.0);
// In thousandths of the picture's shorter side.
const WIDTH: Param = Param::real("width", 1.0, 0.0, 10.0);

#[inline]
fn folded(place: f64) -> f64 {
    let turn = place * 0.5;
    1.0 - (2.0 * (turn - turn.floor()) - 1.0).abs()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Complex {
    re: f64,
    im: f64,
}

const fn complex(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

impl Complex {
    const ZERO: Complex = complex(0.0, 0.0);
    const ONE: Complex = complex(1.0, 0.0);

    #[inline]
    fn norm2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    fn sqrt(self) -> Complex {
        let size = self.norm2().sqrt();
        let re = ((size + self.re) * 0.5).max(0.0).sqrt();
        let im = ((size - self.re) * 0.5).max(0.0).sqrt();
        complex(re, if self.im < 0.0 { -im } else { im })
    }
}

impl Add for Complex {
    type Output = Complex;
    #[inline]
    fn add(self, other: Complex) -> Complex {
        complex(self.re + other.re, self.im + other.im)
    }
}

impl Sub for Complex {
    type Output = Complex;
    #[inline]
    fn sub(self, other: Complex) -> Complex {
        complex(self.re - other.re, self.im - other.im)
    }
}

impl Neg for Complex {
    type Output = Complex;
    #[inline]
    fn neg(self) -> Complex {
        complex(-self.re, -self.im)
    }
}

impl Mul for Complex {
    type Output = Complex;
    #[inline]
    fn mul(self, other: Complex) -> Complex {
        complex(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }
}

impl Mul<f64> for Complex {
    type Output = Complex;
    #[inline]
    fn mul(self, by: f64) -> Complex {
        complex(self.re * by, self.im * by)
    }
}

impl Div for Complex {
    type Output = Complex;
    #[inline]
    fn div(self, other: Complex) -> Complex {
        let size = other.norm2();
        complex(
            (self.re * other.re + self.im * other.im) / size,
            (self.im * other.re - self.re * other.im) / size,
        )
    }
}

// Lines are as bright as a drawing this long, in units of the plane, would be at zoom 1 without the gain.
const LENGTH: f64 = 160.0;
// The view is taken for a disc that holds a picture up to 2.4 times as long as it is high.
const VIEW: f64 = 5.2;

#[derive(Clone, Copy)]
struct Segment {
    from: (f64, f64),
    to: (f64, f64),
    colors: (f32, f32),
}

// Line segments as an orbit. A run spreads its points evenly along the whole length, one at a random place in
// each equal stretch, so it reads the segments in order.
struct Lines {
    segments: Vec<Segment>,
    ends: Vec<f64>,
    // The segment that holds the start of each of twice as many equal stretches of the length.
    firsts: Vec<u32>,
    blur: f64,
    zoom: f64,
}

impl Lines {
    // Only the segments that reach the view are kept, so that every point of a run lands where it can be seen.
    fn new(mut segments: Vec<Segment>, width: f64, camera: &Camera) -> Self {
        let blur = width * 0.001 * 4.0 / camera.zoom;
        let (middle, reach) = (camera.center, 1.25 * VIEW / camera.zoom + blur);
        segments.retain(|segment| {
            let (dx, dy) = (segment.to.0 - segment.from.0, segment.to.1 - segment.from.1);
            let (ox, oy) = (middle[0] - segment.from.0, middle[1] - segment.from.1);
            let length = dx * dx + dy * dy;
            let along = if length > 0.0 {
                ((ox * dx + oy * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (nx, ny) = (ox - dx * along, oy - dy * along);
            nx * nx + ny * ny < reach * reach
        });
        let mut total = 0.0;
        let mut ends: Vec<f64> = segments
            .iter()
            .map(|segment| {
                let (dx, dy) = (segment.to.0 - segment.from.0, segment.to.1 - segment.from.1);
                total += (dx * dx + dy * dy).sqrt();
                total
            })
            .collect();
        if !(total > 0.0 && total.is_finite()) {
            segments.clear();
            ends.clear();
        }
        let stretches = segments.len() * 2;
        let mut firsts = Vec::with_capacity(stretches);
        let mut at = 0usize;
        for stretch in 0..stretches {
            let from = total * stretch as f64 / stretches as f64;
            while at + 1 < ends.len() && ends[at] <= from {
                at += 1;
            }
            firsts.push(at as u32);
        }
        Self {
            segments,
            ends,
            firsts,
            blur,
            zoom: camera.zoom,
        }
    }

    fn len(&self) -> usize {
        self.segments.len()
    }

    fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }
}

struct Pen<'a>(&'a Lines);

impl Orbit for Lines {
    fn walk(&self, _: &mut Rng) -> Box<dyn Walk + '_> {
        Box::new(Pen(self))
    }

    // The points a cell of a line gets fall with the length drawn and with the zoom, not with its square.
    fn gain(&self) -> f64 {
        self.length() / (LENGTH * self.zoom)
    }
}

impl Walk for Pen<'_> {
    fn run(&mut self, rng: &mut Rng, count: u64, plot: &mut dyn FnMut(f64, f64, f64)) {
        let lines = self.0;
        let Some(total) = lines.ends.last().copied() else {
            return;
        };
        let (stretch, last) = (total / count as f64, lines.ends.len() - 1);
        let stretches = lines.firsts.len() as f64 / total;
        let mut at = 0usize;
        for index in 0..count {
            let pick = (index as f64 + rng.next_f64()) * stretch;
            at = at.max(lines.firsts[((pick * stretches) as usize).min(lines.firsts.len() - 1)] as usize);
            while at < last && lines.ends[at] <= pick {
                at += 1;
            }
            let start = if at == 0 { 0.0 } else { lines.ends[at - 1] };
            let segment = &lines.segments[at];
            let length = lines.ends[at] - start;
            let along = if length > 0.0 {
                ((pick - start) / length).min(1.0)
            } else {
                0.0
            };
            let (mut x, mut y) = (
                segment.from.0 + (segment.to.0 - segment.from.0) * along,
                segment.from.1 + (segment.to.1 - segment.from.1) * along,
            );
            if lines.blur > 0.0 {
                // Four 16-bit numbers of one draw: the sum of two is spread like a tent.
                let bits = rng.next_u64();
                let part = |shift: u32| ((bits >> shift) & 0xFFFF) as f64 * (1.0 / 65536.0);
                x += (part(0) + part(16) - 1.0) * lines.blur;
                y += (part(32) + part(48) - 1.0) * lines.blur;
            }
            let color = f64::from(segment.colors.0) + f64::from(segment.colors.1 - segment.colors.0) * along;
            plot(x, y, color);
        }
    }
}

// The share of neighbouring cells of a square probe whose values differ by more than `unit`: 0 is flat, 1 is noise.
fn busy(values: &[Option<f64>], side: usize, unit: f64) -> f64 {
    let differ = |a: usize, b: usize| match (values[a], values[b]) {
        (Some(a), Some(b)) => (a - b).abs() > unit,
        (None, None) => false,
        _ => true,
    };
    let (mut pairs, mut busy) = (0usize, 0usize);
    for y in 0..side {
        for x in 0..side {
            let at = y * side + x;
            if x + 1 < side {
                pairs += 1;
                busy += usize::from(differ(at, at + 1));
            }
            if y + 1 < side {
                pairs += 1;
                busy += usize::from(differ(at, at + side));
            }
        }
    }
    busy as f64 / pairs as f64
}

fn number(fractal: &Fractal, param: &Param) -> f64 {
    fractal.params.get(param.name).copied().unwrap_or(param.default)
}

// A nearby view with the colors moved along the palette.
fn moved(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = fractal.clone();
    let (right, down) = (rng.range(-0.5, 0.5) * strength, rng.range(-0.5, 0.5) * strength);
    out.camera = out.camera.looking_at(right, down, exp(rng.range(-0.8, 1.0) * strength));
    recolored(&mut out, rng, strength);
    out
}

fn recolored(fractal: &mut Fractal, rng: &mut Rng, strength: f64) {
    let shift = number(fractal, &COLOR_SHIFT) + rng.range(0.2, 0.8) * strength;
    let shift = ((shift - 2.0 * (shift * 0.5).floor()) * 1e3).round() / 1e3;
    fractal
        .params
        .insert(COLOR_SHIFT.name.to_owned(), if shift >= 2.0 { 0.0 } else { shift });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{preset, random_scene};
    use crate::scene::Scene;

    #[test]
    fn a_text_is_kept_by_a_scene_and_checked_by_its_kind() {
        let plant = preset("l_system-plant").unwrap();
        let json = plant.to_json();
        assert!(json.contains(r#""axiom": "X""#), "{json}");
        assert_eq!(Scene::from_json(&json).unwrap(), plant);
        let dragon = preset("l_system-dragon").unwrap();
        assert!(!dragon.to_json().contains(r#""text""#));

        let with = |kind: &str, texts: &[(&str, &str)]| {
            let mut scene = plant.clone();
            let fractal = scene.layers[0].fractal_mut().unwrap();
            fractal.kind = kind.to_owned();
            fractal.params.clear();
            fractal.text = texts
                .iter()
                .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
                .collect();
            Scene::from_json(&scene.to_json())
        };
        let own = [("axiom", "F"), ("rules", "F=F[+F]F")];
        with("l_system", &own).unwrap();
        with("lyapunov", &[("sequence", "AAB")]).unwrap();
        let long = "F".repeat(2001);
        for (kind, wrong) in [
            ("l_system", &[("axiom", "")][..]),
            ("l_system", &[("rules", "F")]),
            ("l_system", &[("rules", "F=G,F=F")]),
            ("l_system", &[("words", "F")]),
            ("l_system", &[("axiom", &long)]),
            ("lyapunov", &own),
            ("lyapunov", &[("sequence", "ABC")]),
            ("kleinian", &own),
            ("mandelbrot", &own),
        ] {
            assert!(with(kind, wrong).is_err(), "{kind} {wrong:?}");
        }
        let sequence = random_scene("lyapunov", 2).unwrap().to_json();
        assert!(sequence.contains(r#""sequence": ""#), "{sequence}");
    }

    #[test]
    fn a_run_spreads_its_points_along_the_lines_in_view() {
        let segment = |from: (f64, f64), to: (f64, f64)| Segment {
            from,
            to,
            colors: (0.0, 1.0),
        };
        let camera = Camera::default();
        let far = segment((500.0, 0.0), (600.0, 0.0));
        let lines = Lines::new(
            vec![segment((0.0, 0.0), (1.0, 0.0)), far, segment((1.0, 0.0), (1.0, 3.0))],
            0.0,
            &camera,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines.length(), 4.0);
        let mut rng = Rng::derive(1, 2, 3);
        let (mut first, mut second) = (0, 0);
        lines.walk(&mut rng).run(&mut rng, 4000, &mut |x, y, color| {
            if y == 0.0 {
                first += 1;
                assert!((0.0..=1.0).contains(&x) && (color - x).abs() < 1e-6);
            } else {
                second += 1;
                assert!(x == 1.0 && (0.0..=3.0).contains(&y) && (color - y / 3.0).abs() < 1e-6);
            }
        });
        assert_eq!((first, second), (1000, 3000));
        assert_eq!(lines.gain(), 4.0 / LENGTH);
        let closer = Lines::new(
            vec![segment((0.0, 0.0), (1.0, 0.0))],
            0.0,
            &camera.looking_at(0.0, 0.0, 8.0),
        );
        assert_eq!(closer.gain(), 1.0 / (LENGTH * 8.0));
        let nothing = Lines::new(vec![far], 1.0, &camera);
        nothing
            .walk(&mut rng)
            .run(&mut rng, 10, &mut |_, _, _| panic!("a point of no line"));
        assert_eq!(nothing.gain(), 0.0);
    }
}
