use serde::{Deserialize, Serialize};

use crate::math::{PI, TAU, atan2, cos, cosh, exp, fmod, log10, pow, sin, sinh, tan};
use crate::rng::Rng;

const EPS: f64 = 1e-10;
pub(crate) const MAX_PARAMS: usize = 4;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Linear,
    Sinusoidal,
    Spherical,
    Swirl,
    Horseshoe,
    Polar,
    Handkerchief,
    Heart,
    Disc,
    Spiral,
    Hyperbolic,
    Diamond,
    Ex,
    Julia,
    Bent,
    Fisheye,
    Eyefish,
    Exponential,
    Power,
    Cosine,
    Bubble,
    Cylinder,
    Tangent,
    Cross,
    Blur,
    GaussianBlur,
    Julian,
    Curl,
    Waves,
    Popcorn,
    Rings,
    Fan,
    Blob,
    Pdj,
    Fan2,
    Rings2,
    Perspective,
    Noise,
    Juliascope,
    RadialBlur,
    Pie,
    Ngon,
    Rectangles,
    Arch,
    Square,
    Rays,
    Blade,
    Secant2,
    Twintrian,
}

impl Kind {
    pub const ALL: [Kind; 49] = [
        Kind::Linear,
        Kind::Sinusoidal,
        Kind::Spherical,
        Kind::Swirl,
        Kind::Horseshoe,
        Kind::Polar,
        Kind::Handkerchief,
        Kind::Heart,
        Kind::Disc,
        Kind::Spiral,
        Kind::Hyperbolic,
        Kind::Diamond,
        Kind::Ex,
        Kind::Julia,
        Kind::Bent,
        Kind::Fisheye,
        Kind::Eyefish,
        Kind::Exponential,
        Kind::Power,
        Kind::Cosine,
        Kind::Bubble,
        Kind::Cylinder,
        Kind::Tangent,
        Kind::Cross,
        Kind::Blur,
        Kind::GaussianBlur,
        Kind::Julian,
        Kind::Curl,
        Kind::Waves,
        Kind::Popcorn,
        Kind::Rings,
        Kind::Fan,
        Kind::Blob,
        Kind::Pdj,
        Kind::Fan2,
        Kind::Rings2,
        Kind::Perspective,
        Kind::Noise,
        Kind::Juliascope,
        Kind::RadialBlur,
        Kind::Pie,
        Kind::Ngon,
        Kind::Rectangles,
        Kind::Arch,
        Kind::Square,
        Kind::Rays,
        Kind::Blade,
        Kind::Secant2,
        Kind::Twintrian,
    ];

    pub fn params(self) -> &'static [(&'static str, f64)] {
        match self {
            Kind::Julian => &[("power", 2.0), ("dist", 1.0)],
            Kind::Curl => &[("c1", 0.5), ("c2", 0.2)],
            Kind::Blob => &[("low", 0.5), ("high", 1.0), ("waves", 5.0)],
            Kind::Pdj => &[("a", 1.5), ("b", 1.8), ("c", 1.2), ("d", 2.0)],
            Kind::Fan2 => &[("x", 0.5), ("y", 0.3)],
            Kind::Rings2 => &[("val", 0.5)],
            Kind::Perspective => &[("angle", 0.5), ("dist", 2.0)],
            Kind::Juliascope => &[("power", 2.0), ("dist", 1.0)],
            Kind::RadialBlur => &[("angle", 0.5)],
            Kind::Pie => &[("slices", 6.0), ("rotation", 0.0), ("thickness", 0.5)],
            Kind::Ngon => &[("sides", 5.0), ("power", 2.0), ("circle", 1.0), ("corners", 1.0)],
            Kind::Rectangles => &[("x", 0.5), ("y", 0.5)],
            _ => &[],
        }
    }

    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    pub(crate) fn resolve(self, given: &[f64]) -> [f64; MAX_PARAMS] {
        let mut out = [0.0; MAX_PARAMS];
        for (i, (_, default)) in self.params().iter().enumerate() {
            out[i] = given.get(i).copied().unwrap_or(*default);
        }
        out
    }

    #[inline]
    pub(crate) fn eval(
        self,
        x: f64,
        y: f64,
        p: &[f64; MAX_PARAMS],
        weight: f64,
        affine: &[f64; 6],
        rng: &mut Rng,
    ) -> (f64, f64) {
        match self {
            Kind::Linear => (x, y),
            Kind::Sinusoidal => (sin(x), sin(y)),
            Kind::Spherical => {
                let k = 1.0 / (x * x + y * y + EPS);
                (x * k, y * k)
            }
            Kind::Swirl => {
                let r2 = x * x + y * y;
                let (s, c) = (sin(r2), cos(r2));
                (x * s - y * c, x * c + y * s)
            }
            Kind::Horseshoe => {
                let k = 1.0 / ((x * x + y * y).sqrt() + EPS);
                ((x - y) * (x + y) * k, 2.0 * x * y * k)
            }
            Kind::Polar => (atan2(x, y) / PI, (x * x + y * y).sqrt() - 1.0),
            Kind::Handkerchief => {
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                (r * sin(t + r), r * cos(t - r))
            }
            Kind::Heart => {
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                (r * sin(t * r), -r * cos(t * r))
            }
            Kind::Disc => {
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y) / PI);
                (t * sin(PI * r), t * cos(PI * r))
            }
            Kind::Spiral => {
                let (r, t) = ((x * x + y * y).sqrt() + EPS, atan2(x, y));
                ((cos(t) + sin(r)) / r, (sin(t) - cos(r)) / r)
            }
            Kind::Hyperbolic => {
                let (r, t) = ((x * x + y * y).sqrt() + EPS, atan2(x, y));
                (sin(t) / r, r * cos(t))
            }
            Kind::Diamond => {
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                (sin(t) * cos(r), cos(t) * sin(r))
            }
            Kind::Ex => {
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let (p0, p1) = (sin(t + r), cos(t - r));
                let (m0, m1) = (p0 * p0 * p0, p1 * p1 * p1);
                (r * (m0 + m1), r * (m0 - m1))
            }
            Kind::Julia => {
                let r = (x * x + y * y).sqrt().sqrt();
                let t = atan2(x, y) * 0.5 + if rng.next_bool() { PI } else { 0.0 };
                (r * cos(t), r * sin(t))
            }
            Kind::Bent => (if x < 0.0 { 2.0 * x } else { x }, if y < 0.0 { 0.5 * y } else { y }),
            Kind::Fisheye => {
                let k = 2.0 / ((x * x + y * y).sqrt() + 1.0);
                (k * y, k * x)
            }
            Kind::Eyefish => {
                let k = 2.0 / ((x * x + y * y).sqrt() + 1.0);
                (k * x, k * y)
            }
            Kind::Exponential => {
                let k = exp(x - 1.0);
                (k * cos(PI * y), k * sin(PI * y))
            }
            Kind::Power => {
                let t = atan2(x, y);
                let (s, c) = (sin(t), cos(t));
                let k = pow((x * x + y * y).sqrt(), s);
                (k * c, k * s)
            }
            Kind::Cosine => (cos(PI * x) * cosh(y), -sin(PI * x) * sinh(y)),
            Kind::Bubble => {
                let k = 4.0 / (x * x + y * y + 4.0);
                (k * x, k * y)
            }
            Kind::Cylinder => (sin(x), y),
            Kind::Tangent => (sin(x) / (cos(y) + EPS), tan(y)),
            Kind::Cross => {
                let s = x * x - y * y;
                let k = (1.0 / (s * s + EPS)).sqrt();
                (k * x, k * y)
            }
            Kind::Blur => {
                let (r, a) = (rng.next_f64(), rng.next_f64() * TAU);
                (r * cos(a), r * sin(a))
            }
            Kind::GaussianBlur => {
                let r = rng.next_f64() + rng.next_f64() + rng.next_f64() + rng.next_f64() - 2.0;
                let a = rng.next_f64() * TAU;
                (r * cos(a), r * sin(a))
            }
            Kind::Julian => {
                let (power, dist) = (p[0], p[1]);
                let branch = (power.abs() * rng.next_f64()).trunc();
                let a = (atan2(y, x) + TAU * branch) / power;
                let r = pow(x * x + y * y + EPS, dist / power * 0.5);
                (r * cos(a), r * sin(a))
            }
            Kind::Curl => {
                let (c1, c2) = (p[0], p[1]);
                let re = 1.0 + c1 * x + c2 * (x * x - y * y);
                let im = c1 * y + 2.0 * c2 * x * y;
                let k = 1.0 / (re * re + im * im + EPS);
                ((x * re + y * im) * k, (y * re - x * im) * k)
            }
            Kind::Waves => {
                let [_, b, c, _, e, f] = *affine;
                (x + b * sin(y / (c * c + EPS)), y + e * sin(x / (f * f + EPS)))
            }
            Kind::Popcorn => {
                let (c, f) = (affine[2], affine[5]);
                (x + c * sin(tan(3.0 * y)), y + f * sin(tan(3.0 * x)))
            }
            Kind::Rings => {
                let c2 = affine[2] * affine[2] + EPS;
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let k = fmod(r + c2, 2.0 * c2) - c2 + r * (1.0 - c2);
                (k * cos(t), k * sin(t))
            }
            Kind::Fan => {
                let half = PI * (affine[2] * affine[2] + EPS) * 0.5;
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let a = if fmod(t + affine[5], 2.0 * half) > half {
                    t - half
                } else {
                    t + half
                };
                (r * cos(a), r * sin(a))
            }
            Kind::Blob => {
                let (low, high, waves) = (p[0], p[1], p[2]);
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let k = r * (low + (high - low) * 0.5 * (sin(waves * t) + 1.0));
                (k * cos(t), k * sin(t))
            }
            Kind::Pdj => (sin(p[0] * y) - cos(p[1] * x), sin(p[2] * x) - cos(p[3] * y)),
            Kind::Fan2 => {
                let (wedge, shift) = (PI * p[0] * p[0] + EPS, p[1]);
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let within = t + shift - wedge * (2.0 * t * shift / wedge).trunc();
                let a = if within > wedge * 0.5 {
                    t - wedge * 0.5
                } else {
                    t + wedge * 0.5
                };
                (r * sin(a), r * cos(a))
            }
            Kind::Rings2 => {
                let v = p[0] * p[0] + EPS;
                let (r, t) = ((x * x + y * y).sqrt(), atan2(x, y));
                let k = r - 2.0 * v * ((r + v) / (2.0 * v)).trunc() + r * (1.0 - v);
                (k * sin(t), k * cos(t))
            }
            Kind::Perspective => {
                let (a, dist) = (p[0] * PI * 0.5, p[1]);
                let k = dist / (dist - y * sin(a));
                (k * x, k * y * cos(a))
            }
            Kind::Noise => {
                let (r, a) = (rng.next_f64(), rng.next_f64() * TAU);
                (r * x * cos(a), r * y * sin(a))
            }
            Kind::Juliascope => {
                let (power, dist) = (p[0], p[1]);
                let branch = (power.abs() * rng.next_f64()).trunc();
                let phi = atan2(y, x);
                let side = if fmod(branch, 2.0) == 0.0 { phi } else { -phi };
                let a = (side + TAU * branch) / power;
                let r = pow(x * x + y * y + EPS, dist / power * 0.5);
                (r * cos(a), r * sin(a))
            }
            Kind::RadialBlur => {
                let spread = weight * (rng.next_f64() + rng.next_f64() + rng.next_f64() + rng.next_f64() - 2.0);
                if weight == 0.0 {
                    return (0.0, 0.0);
                }
                let a = p[0] * PI * 0.5;
                let r = (x * x + y * y).sqrt();
                let turned = atan2(y, x) + spread * sin(a);
                let zoom = spread * cos(a) - 1.0;
                (
                    (r * cos(turned) + zoom * x) / weight,
                    (r * sin(turned) + zoom * y) / weight,
                )
            }
            Kind::Pie => {
                let (slices, rotation, thickness) = (p[0], p[1], p[2]);
                let slice = (rng.next_f64() * slices + 0.5).trunc();
                let a = rotation + TAU * (slice + rng.next_f64() * thickness) / slices;
                let r = rng.next_f64();
                (r * cos(a), r * sin(a))
            }
            Kind::Ngon => {
                let (side, power, circle, corners) = (TAU / p[0], p[1], p[2], p[3]);
                let phi = atan2(y, x);
                let mut within = phi - side * (phi / side).floor();
                if within > side * 0.5 {
                    within -= side;
                }
                let k = (corners * (1.0 / cos(within) - 1.0) + circle) / pow(x * x + y * y + EPS, power * 0.5);
                (k * x, k * y)
            }
            Kind::Rectangles => {
                let fold = |v: f64, size: f64| {
                    if size == 0.0 {
                        v
                    } else {
                        (2.0 * (v / size).floor() + 1.0) * size - v
                    }
                };
                (fold(x, p[0]), fold(y, p[1]))
            }
            Kind::Arch => {
                let a = rng.next_f64() * PI * weight;
                let s = sin(a);
                (s, s * s / cos(a))
            }
            Kind::Square => (rng.next_f64() - 0.5, rng.next_f64() - 0.5),
            Kind::Rays => {
                let k = weight * tan(rng.next_f64() * PI * weight) / (x * x + y * y + EPS);
                (k * cos(x), k * sin(y))
            }
            Kind::Blade => {
                let a = rng.next_f64() * weight * (x * x + y * y).sqrt();
                let (s, c) = (sin(a), cos(a));
                (x * (c + s), x * (c - s))
            }
            Kind::Secant2 => {
                let c = cos(weight * (x * x + y * y).sqrt());
                (x, if c < 0.0 { 1.0 / c + 1.0 } else { 1.0 / c - 1.0 })
            }
            Kind::Twintrian => {
                let a = rng.next_f64() * weight * (x * x + y * y).sqrt();
                let (s, c) = (sin(a), cos(a));
                // log10(0) is clamped to -30, as flam3 does, so the origin stays a point.
                let k = if s == 0.0 { -30.0 } else { log10(s * s) } + c;
                (x * k, x * (k - PI * s))
            }
        }
    }
}

fn one() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Variation {
    #[serde(rename = "type")]
    pub kind: Kind,
    #[serde(default = "one")]
    pub weight: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<f64>,
}

impl Variation {
    pub fn new(kind: Kind, weight: f64) -> Self {
        Self {
            kind,
            weight,
            params: Vec::new(),
        }
    }

    pub fn with_params(kind: Kind, weight: f64, params: &[f64]) -> Self {
        Self {
            kind,
            weight,
            params: params.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_listed_once() {
        for (i, kind) in Kind::ALL.iter().enumerate() {
            assert_eq!(Kind::ALL.iter().position(|k| k == kind), Some(i));
            assert!(!kind.name().is_empty());
        }
    }

    #[test]
    fn ordinary_points_stay_finite() {
        let mut rng = Rng::derive(1, 2, 3);
        for kind in Kind::ALL {
            let p = kind.resolve(&[]);
            for (x, y) in [(0.3, -0.7), (-1.2, 0.4), (0.0, 0.0), (2.0, 2.0)] {
                let (ox, oy) = kind.eval(x, y, &p, 1.0, &AFFINE, &mut rng);
                assert!(ox.is_finite() && oy.is_finite(), "{kind:?} at ({x}, {y})");
            }
        }
    }
    const AFFINE: [f64; 6] = [0.5, -0.4, 0.6, 0.3, 0.7, -0.8];

    // The expected values come from the formulas of docs/scene-format.md worked out apart from this crate.
    fn known(kind: Kind, cases: [((f64, f64), (f64, f64)); 2]) {
        for ((x, y), (want_x, want_y)) in cases {
            let mut rng = Rng::derive(1, 2, 3);
            let (got_x, got_y) = kind.eval(x, y, &kind.resolve(&[]), 0.8, &AFFINE, &mut rng);
            assert!(
                (got_x - want_x).abs() < 1e-9 && (got_y - want_y).abs() < 1e-9,
                "{kind:?} at ({x}, {y}): got ({got_x}, {got_y})"
            );
        }
    }

    #[test]
    fn waves_gives_its_known_values() {
        known(
            Kind::Waves,
            [
                ((0.3, -0.7), (0.6724007716139999, -0.3837599700015607)),
                ((-1.2, 0.4), (-1.5584768803572089, -0.26786004718821355)),
            ],
        );
    }

    #[test]
    fn popcorn_gives_its_known_values() {
        known(
            Kind::Popcorn,
            [
                ((0.3, -0.7), (0.8942088512180111, -1.4617109722205355)),
                ((-1.2, 0.4), (-0.8765031790202399, 0.7789454910261773)),
            ],
        );
    }

    #[test]
    fn rings_gives_its_known_values() {
        known(
            Kind::Rings,
            [
                ((0.3, -0.7), (-0.4862155781331692, 0.2083781049142154)),
                ((-1.2, 0.4), (0.20063201676926232, -0.6018960503077869)),
            ],
        );
    }

    #[test]
    fn fan_gives_its_known_values() {
        known(
            Kind::Fan,
            [
                ((0.3, -0.7), (-0.43028150925900627, 0.6283771342034906)),
                ((-1.2, 0.4), (0.980723324301087, -0.7988627924567678)),
            ],
        );
    }

    #[test]
    fn blob_gives_its_known_values() {
        known(
            Kind::Blob,
            [
                ((0.3, -0.7), (-0.6822983929195587, 0.2924135969655252)),
                ((-1.2, 0.4), (0.30379473319220207, -0.9113841995766061)),
            ],
        );
    }

    #[test]
    fn pdj_gives_its_known_values() {
        known(
            Kind::Pdj,
            [
                ((0.3, -0.7), (-1.725131906957841, 0.1823070903748489)),
                ((-1.2, 0.4), (1.1203416196456482, -1.6881650575388518)),
            ],
        );
    }

    #[test]
    fn fan2_gives_its_known_values() {
        known(
            Kind::Fan2,
            [
                ((0.3, -0.7), (0.5450422624355444, -0.5319106430211216)),
                ((-1.2, 0.4), (-0.9555820660260697, 0.8287719318904014)),
            ],
        );
    }

    #[test]
    fn rings2_gives_its_known_values() {
        known(
            Kind::Rings2,
            [
                ((0.3, -0.7), (0.13108070123326457, -0.305854969544284)),
                ((-1.2, 0.4), (-0.6769750522350193, 0.22565835074500645)),
            ],
        );
    }

    #[test]
    fn perspective_gives_its_known_values() {
        known(
            Kind::Perspective,
            [
                ((0.3, -0.7), (0.24048339597913448, -0.39677736013910336)),
                ((-1.2, 0.4), (-1.3976588035558892, 0.3294313392598153)),
            ],
        );
    }

    #[test]
    fn noise_gives_its_known_values() {
        known(
            Kind::Noise,
            [
                ((0.3, -0.7), (0.11043869554980446, 0.16819892366077255)),
                ((-1.2, 0.4), (-0.4417547821992178, -0.09611367066329862)),
            ],
        );
    }

    #[test]
    fn juliascope_gives_its_known_values() {
        known(
            Kind::Juliascope,
            [
                ((0.3, -0.7), (0.7285524382904454, -0.4804046786960874)),
                ((-1.2, 0.4), (0.18015418961181592, 1.1101601380306312)),
            ],
        );
    }

    #[test]
    fn radial_blur_gives_its_known_values() {
        known(
            Kind::RadialBlur,
            [
                ((0.3, -0.7), (0.620086315002564, -0.15741152070339617)),
                ((-1.2, 0.4), (-0.8796405801863557, -0.5960668509511592)),
            ],
        );
    }

    #[test]
    fn pie_gives_its_known_values() {
        known(
            Kind::Pie,
            [
                ((0.3, -0.7), (-0.6522151690096311, -0.33575305140213846)),
                ((-1.2, 0.4), (-0.6522151690096311, -0.33575305140213846)),
            ],
        );
    }

    #[test]
    fn ngon_gives_its_known_values() {
        known(
            Kind::Ngon,
            [
                ((0.3, -0.7), (0.5193777732671334, -1.2118814709566446)),
                ((-1.2, 0.4), (-0.7866789635589475, 0.26222632118631584)),
            ],
        );
    }

    #[test]
    fn rectangles_gives_its_known_values() {
        known(
            Kind::Rectangles,
            [((0.3, -0.7), (0.2, -0.8)), ((-1.2, 0.4), (-1.3, 0.09999999999999998))],
        );
    }

    #[test]
    fn arch_gives_its_known_values() {
        known(
            Kind::Arch,
            [
                ((0.3, -0.7), (0.8933995520172595, 1.7766047168588717)),
                ((-1.2, 0.4), (0.8933995520172595, 1.7766047168588717)),
            ],
        );
    }

    #[test]
    fn square_gives_its_known_values() {
        known(
            Kind::Square,
            [
                ((0.3, -0.7), (-0.06039172520107017, 0.4079633837685236)),
                ((-1.2, 0.4), (-0.06039172520107017, 0.4079633837685236)),
            ],
        );
    }

    #[test]
    fn rays_gives_its_known_values() {
        known(
            Kind::Rays,
            [
                ((0.3, -0.7), (2.620374954324528, -1.7670128923009976)),
                ((-1.2, 0.4), (0.3602903618542305, 0.38719655845557516)),
            ],
        );
    }

    #[test]
    fn blade_gives_its_known_values() {
        known(
            Kind::Blade,
            [
                ((0.3, -0.7), (0.3686974333784107, 0.20990998694719695)),
                ((-1.2, 0.4), (-1.5995985510552477, -0.5668196145705902)),
            ],
        );
    }

    #[test]
    fn secant2_gives_its_known_values() {
        known(
            Kind::Secant2,
            [
                ((0.3, -0.7), (0.3, 0.2194071216358846)),
                ((-1.2, 0.4), (-1.2, 0.8859870995547667)),
            ],
        );
    }

    #[test]
    fn twintrian_gives_its_known_values() {
        known(
            Kind::Twintrian,
            [
                ((0.3, -0.7), (-0.057097341317870316, -0.30652007891316235)),
                ((-1.2, 0.4), (-0.20431979530678043, 1.4179655645144593)),
            ],
        );
    }

    #[test]
    fn the_affine_and_the_weight_reach_only_the_variations_that_use_them() {
        let other = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        for kind in &Kind::ALL[..28] {
            let p = kind.resolve(&[]);
            let first = kind.eval(0.3, -0.7, &p, 0.8, &AFFINE, &mut Rng::derive(1, 2, 3));
            let second = kind.eval(0.3, -0.7, &p, 0.1, &other, &mut Rng::derive(1, 2, 3));
            assert_eq!(first, second, "{kind:?}");
        }
    }
}
