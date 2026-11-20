use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, TONE, candidate, limit, moved};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, stream};
use crate::scene::{Camera, Fractal};
use crate::fractal::Param;
use crate::math::ln;
use super::{div, mul, norm};

const NAME: &str = "magnet";
const ATTEMPTS: u64 = 24;
const WHOLE: [Camera; 2] = [Camera {
    center: [1.35, 0.0],
    zoom: 0.95,
    rotation: 0.0,
    flip_y: false,
}, Camera {
    center: [1.0, 0.0],
    zoom: 1.3,
    rotation: 0.0,
    flip_y: false,
}];
// The orbit is taken to have settled on 1 when it is this near, squared.
const NEAR: f64 = 1e-8;

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[
        Param::whole("type", 1.0, 1.0, 2.0).nearest(),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        EDGE,
        FADE,
    ],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Magnet::new(params, camera))),
    random,
    mutate: moved,
    presets: &[
        Preset {
            name: "first",
            palette: "ice",
            center: [1.35, 0.0],
            zoom: 0.95,
            params: &[("color_density", 1.3), ("color_shift", 1.0)],
        },
        Preset {
            name: "second",
            palette: "gold",
            center: [1.0, 0.0],
            zoom: 1.3,
            params: &[("type", 2.0), ("color_density", 1.3), ("color_shift", 0.4)],
        },
        Preset {
            name: "foam",
            palette: "ice",
            center: [1.868274, 0.781432],
            zoom: 22.0,
            params: &[("iterations", 300.0), ("color_density", 1.61), ("color_shift", 1.25)],
        },
        Preset {
            name: "pearls",
            palette: "violet",
            center: [1.88699018, -1.04715939],
            zoom: 400.0,
            params: &[("iterations", 300.0), ("color_density", 1.43), ("color_shift", 0.68)],
        },
        Preset {
            name: "coral",
            palette: "sunset",
            center: [1.77143297, -0.39961021],
            zoom: 44.0,
            params: &[("type", 2.0), ("iterations", 300.0), ("color_density", 2.26), ("color_shift", 1.83)],
        },
        Preset {
            name: "cells",
            palette: "emerald",
            center: [2.15000135, 0.79235138],
            zoom: 260.0,
            params: &[("type", 2.0), ("iterations", 300.0), ("color_density", 3.18), ("color_shift", 0.95)],
        },
    ],
};

struct Magnet {
    second: bool,
    limit: u32,
    far: Coloring,
    near: Coloring,
}

impl Magnet {
    fn new(params: &[f64], camera: &Camera) -> Self {
        let second = params[0] >= 1.5;
        let far = Coloring::new(params[2], params[3], params[4], params[5], camera.zoom);
        Self {
            second,
            limit: limit(params[1], camera.zoom),
            far,
            near: if second { far.of_power(3.0) } else { far },
        }
    }
}

impl Field for Magnet {
    #[inline]
    fn sample(&self, cx: f64, cy: f64) -> Option<Sample> {
        let (c1, c2) = ((cx - 1.0, cy), (cx - 2.0, cy));
        let product = mul(c1, c2);
        let slope = (2.0 * cx - 3.0, 2.0 * cy);
        // Far away the map is z² / 4 or z² / 9, and next to 1 it is 1 + 2 (z - 1)² / c or 1 + 2 (z - 1)³ / c².
        let (far, near) = if self.second {
            (81.0, 2.0 / (cx * cx + cy * cy))
        } else {
            (16.0, 4.0 / (cx * cx + cy * cy))
        };
        let (mut z, mut dz) = ((0.0f64, 0.0f64), (0.0f64, 0.0f64));
        let (mut old, mut lap, mut period) = ((f64::NAN, f64::NAN), 0u32, 8u32);
        for step in 0..self.limit {
            let r2 = norm(z);
            if r2 > ESCAPE {
                return Some(self.far.outside(step, r2 / far, norm(dz) / far));
            }
            let e2 = norm((z.0 - 1.0, z.1));
            if e2 < NEAR {
                let e2 = e2.max(1e-300);
                let half_log = (-0.5 * ln(e2 * near)).max(1e-3);
                return Some(self.near.settled(step, half_log, e2, norm(dz)));
            }
            let z2 = mul(z, z);
            let (top, bottom, dtop, dbottom) = if self.second {
                let (lean, tilt) = (mul(c1, z), mul(c2, z));
                let (dz2, three) = (mul(z2, dz), (3.0 * z.0 + slope.0, 3.0 * z.1 + slope.1));
                let (dlean, dtilt) = (mul(c1, dz), mul(c2, dz));
                let zdz = mul(z, dz);
                let z3 = mul(z2, z);
                (
                    (z3.0 + 3.0 * lean.0 + product.0, z3.1 + 3.0 * lean.1 + product.1),
                    (
                        3.0 * z2.0 + 3.0 * tilt.0 + product.0 + 1.0,
                        3.0 * z2.1 + 3.0 * tilt.1 + product.1,
                    ),
                    (3.0 * dz2.0 + 3.0 * dlean.0 + three.0, 3.0 * dz2.1 + 3.0 * dlean.1 + three.1),
                    (6.0 * zdz.0 + 3.0 * dtilt.0 + three.0, 6.0 * zdz.1 + 3.0 * dtilt.1 + three.1),
                )
            } else {
                let zdz = mul(z, dz);
                (
                    (z2.0 + c1.0, z2.1 + c1.1),
                    (2.0 * z.0 + c2.0, 2.0 * z.1 + c2.1),
                    (2.0 * zdz.0 + 1.0, 2.0 * zdz.1),
                    (2.0 * dz.0 + 1.0, 2.0 * dz.1),
                )
            };
            if norm(bottom) == 0.0 {
                return None;
            }
            let ratio = div(top, bottom);
            let lost = mul(ratio, dbottom);
            let dratio = div((dtop.0 - lost.0, dtop.1 - lost.1), bottom);
            let twice = mul(ratio, dratio);
            dz = (2.0 * twice.0, 2.0 * twice.1);
            z = mul(ratio, ratio);
            if z == old {
                return None;
            }
            lap += 1;
            if lap == period {
                (old, lap, period) = (z, 0, period.saturating_mul(2));
            }
        }
        None
    }
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let second = rng.next_f64() < 0.4;
        let kind = if second { 2.0 } else { 1.0 };
        let (camera, colors, params) = candidate(&mut rng, &WHOLE[usize::from(second)], 7, &|shared, camera| {
            Box::new(Magnet::new(&[&[kind][..], &shared[..]].concat(), camera))
        });
        let kind = [("type", kind)];
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &[&kind[..], &params[..]].concat()),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &kind)),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", WHOLE[0].clone(), &[]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn points_escape_settle_on_one_or_stay() {
        // In order: a point of the set, one whose orbit escapes and one whose orbit settles on 1.
        for (kind, points) in [
            (1.0, [(0.5, 0.0), (2.2, 0.3), (-3.0, 2.5)]),
            (2.0, [(0.5, 0.0), (1.5, 1.0), (3.0, 1.0)]),
        ] {
            let magnet = Magnet::new(&[kind, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
            let [stays, far, near] = points.map(|(x, y)| magnet.sample(x, y));
            assert!(stays.is_none(), "{kind}");
            for sample in [far, near] {
                let sample = sample.expect("the orbit ends");
                assert!(sample.at > 0.0 && sample.at < 4.0 && sample.light == 1.0, "{kind}: {}", sample.at);
            }
        }
        // With c = 3 the first map sends 0 to 4, and 4 to itself.
        let first = Magnet::new(&[1.0, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(first.sample(3.0, 0.0).is_none());
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("magnet-first").unwrap());
        assert_eq!(hash, 0x5b7e_db6d_74c7_7612, "got {hash:#018x}");
    }
}
