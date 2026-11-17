use serde::{Deserialize, Serialize};

use crate::math::{PI, TAU, atan2, cos, cosh, exp, pow, sin, sinh, tan};
use crate::rng::Rng;

const EPS: f64 = 1e-10;

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
}

impl Kind {
    pub const ALL: [Kind; 28] = [
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
    ];

    /// Names and defaults of the positional `params` of this variation.
    pub fn params(self) -> &'static [(&'static str, f64)] {
        match self {
            Kind::Julian => &[("power", 2.0), ("dist", 1.0)],
            Kind::Curl => &[("c1", 0.5), ("c2", 0.2)],
            _ => &[],
        }
    }

    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    pub(crate) fn resolve(self, given: &[f64]) -> [f64; 2] {
        let mut out = [0.0; 2];
        for (i, (_, default)) in self.params().iter().enumerate() {
            out[i] = given.get(i).copied().unwrap_or(*default);
        }
        out
    }

    #[inline]
    pub(crate) fn eval(self, x: f64, y: f64, p: &[f64; 2], rng: &mut Rng) -> (f64, f64) {
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
    /// Positional, see [`Kind::params`]; missing ones take their defaults.
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
                let (ox, oy) = kind.eval(x, y, &p, &mut rng);
                assert!(ox.is_finite() && oy.is_finite(), "{kind:?} at ({x}, {y})");
            }
        }
    }
}
