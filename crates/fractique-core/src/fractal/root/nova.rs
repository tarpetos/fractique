use super::{COLOR_DENSITY, COLOR_SHIFT, Complex, ITERATIONS, RELAXATION, SHADE, TONE, div, light, mul, norm, part};
use crate::fractal::escape::{candidate, descend, judged, moved, nudged, value};
use crate::fractal::{Draw, Family, Field, Param, Preset, Sample, Spec, random_palette, stream};
use crate::math::{ln_1p, pow};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};

const NAME: &str = "nova";
const ATTEMPTS: u64 = 24;
// An orbit has settled when a step moves it less than this, squared.
const NEAR: f64 = 1e-10;
const WHOLE: Camera = Camera {
    center: [-0.4, 0.0],
    zoom: 1.6,
    rotation: 0.0,
    flip_y: false,
};

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Root,
    params: &[
        Param::whole("form", 0.0, 0.0, 1.0).nearest(),
        Param::whole("degree", 3.0, 2.0, 8.0),
        RELAXATION,
        Param::real("real", 0.3, -2.0, 2.0),
        Param::real("imaginary", 0.0, -2.0, 2.0),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        SHADE,
    ],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Nova::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "whole",
            palette: "sunset",
            center: [-0.3, 0.0],
            zoom: 2.4,
            params: &[("iterations", 400.0), ("color_density", 2.2), ("color_shift", 1.0), ("shade", 0.4)],
        },
        Preset {
            name: "julia",
            palette: "violet",
            center: [0.0, 0.0],
            zoom: 1.3,
            params: &[("form", 1.0), ("real", -0.3359), ("imaginary", 0.1722), ("iterations", 1800.0), ("color_density", 1.37), ("color_shift", 0.48), ("shade", 0.5)],
        },
        Preset {
            name: "star",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("form", 1.0), ("degree", 6.0), ("real", 0.0177), ("imaginary", 0.0042), ("iterations", 350.0), ("color_density", 1.46), ("color_shift", 1.49), ("shade", 0.39)],
        },
        Preset {
            name: "garden",
            palette: "emerald",
            center: [0.0, 0.0],
            zoom: 1.3,
            params: &[("form", 1.0), ("degree", 5.0), ("real", 0.1458), ("imaginary", 0.0379), ("iterations", 310.0), ("color_density", 2.16), ("color_shift", 0.32), ("shade", 0.22)],
        },
        Preset {
            name: "sapphire",
            palette: "ice",
            center: [-0.03334, -0.45153],
            zoom: 3.7,
            params: &[("form", 1.0), ("degree", 4.0), ("real", -0.306), ("imaginary", 0.0781), ("iterations", 500.0), ("color_density", 0.53), ("color_shift", 1.88), ("shade", 0.57)],
        },
        Preset {
            name: "lace",
            palette: "ice",
            center: [-0.113405014, -0.240815496],
            zoom: 560.0,
            params: &[("degree", 5.0), ("relaxation", 1.4), ("iterations", 200.0), ("color_density", 3.13), ("color_shift", 1.14), ("shade", 0.29)],
        },
    ],
};

struct Nova {
    julia: bool,
    degree: u32,
    relaxation: f64,
    c: Complex,
    // Where the derivative of the step is zero: the orbit that tells what a constant does.
    start: Complex,
    limit: u32,
    density: f64,
    shift: f64,
    shade: f64,
}

impl Nova {
    fn new(params: &[f64], camera: &Camera) -> Self {
        let (degree, relaxation) = (params[1], params[2]);
        Self {
            julia: params[0] >= 0.5,
            degree: degree as u32,
            relaxation,
            c: (params[3], params[4]),
            start: (pow(relaxation * (degree - 1.0) / (degree - relaxation), 1.0 / degree), 0.0),
            limit: crate::fractal::escape::limit(params[5], camera.zoom),
            density: params[6],
            shift: params[7],
            shade: params[8],
        }
    }
}

impl Field for Nova {
    #[inline]
    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        let (c, mut z) = if self.julia { (self.c, (x, y)) } else { ((x, y), self.start) };
        let scale = self.relaxation / f64::from(self.degree);
        let mut before = f64::INFINITY;
        let (mut old, mut lap, mut period) = ((f64::NAN, f64::NAN), 0u32, 8u32);
        for step in 0..self.limit {
            let mut low = z;
            for _ in 2..self.degree {
                low = mul(low, z);
            }
            let full = mul(low, z);
            if norm(low) == 0.0 {
                return None;
            }
            let off = div((full.0 - 1.0, full.1), low);
            let next = (z.0 - scale * off.0 + c.0, z.1 - scale * off.1 + c.1);
            let moved = norm((next.0 - z.0, next.1 - z.1));
            if moved < NEAR {
                let count = f64::from(step) + part(before, moved, NEAR, false);
                return Some(Sample {
                    at: self.shift + self.density * ln_1p(count),
                    light: light(count, self.shade, 40.0),
                });
            }
            (before, z) = (moved, next);
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

fn field(form: f64, degree: f64, relaxation: f64, c: Complex) -> impl Fn(&[f64; 5], &Camera) -> Box<dyn Field> {
    move |shared, camera| {
        Box::new(Nova::new(
            &[form, degree, relaxation, c.0, c.1, shared[0], shared[1], shared[2], 0.0],
            camera,
        ))
    }
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let degree = if rng.next_f64() < 0.55 { 3.0 } else { 2.0 + rng.below(5) as f64 };
        let relaxation = if rng.next_f64() < 0.5 {
            1.0
        } else {
            (rng.range(0.6, 1.8) * 100.0).round() / 100.0
        };
        let map = field(0.0, degree, relaxation, (0.0, 0.0));
        let mut own = vec![("degree", degree), ("relaxation", relaxation)];
        let (camera, colors, params) = if rng.next_f64() < 0.6 {
            candidate(&mut rng, &WHOLE, 6, &map)
        } else {
            // The sets worth a picture belong to the points along the edge of the map of the first form.
            let steps = 2 + rng.below(4);
            let c = descend(&mut rng, steps, &WHOLE, &map).center.map(|part| (part * 1e4).round() / 1e4);
            let mut camera = Camera {
                zoom: rng.range(1.0, 1.5),
                ..Camera::default()
            };
            if rng.next_f64() < 0.3 {
                camera = camera.looking_at(rng.range(-0.2, 0.2), rng.range(-0.2, 0.2), rng.range(1.5, 4.0));
            }
            let colors = random_palette(&mut rng);
            own.extend([("form", 1.0), ("real", c[0]), ("imaginary", c[1])]);
            let params = judged(&mut rng, &camera, &field(1.0, degree, relaxation, (c[0], c[1])));
            (camera, colors, params)
        };
        own.push(("shade", (rng.range(0.2, 0.7) * 100.0).round() / 100.0));
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &[&own[..], &params[..]].concat()),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &own)),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("gold", WHOLE, &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    if value(&SPEC, fractal, "form") < 0.5 {
        return moved(fractal, rng, strength);
    }
    let mut out = moved(fractal, rng, strength * 0.3);
    nudged(&SPEC, &mut out, "real", rng, 0.05 * strength);
    nudged(&SPEC, &mut out, "imaginary", rng, 0.05 * strength);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn the_first_form_starts_where_the_step_stands_still() {
        for (degree, relaxation) in [(3.0, 1.0), (2.0, 1.5), (5.0, 0.7), (8.0, 1.9)] {
            let nova = Nova::new(&[0.0, degree, relaxation, 0.0, 0.0, 100.0, 1.0, 0.0, 0.0], &Camera::default());
            let step = |x: f64| {
                let low = crate::math::pow(x, degree - 1.0);
                x - relaxation * (low * x - 1.0) / (degree * low)
            };
            let slope = (step(nova.start.0 + 1e-6) - step(nova.start.0 - 1e-6)) / 2e-6;
            assert!(slope.abs() < 1e-5, "{degree} {relaxation}: {slope}");
        }
        let nova = Nova::new(&[0.0, 3.0, 1.0, 0.0, 0.0, 100.0, 1.0, 0.0, 0.0], &Camera::default());
        assert_eq!(nova.start, (1.0, 0.0));
        let still = nova.sample(0.0, 0.0).expect("z = 1 is a root");
        assert!(still.at < 1.0);
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("nova-whole").unwrap());
        assert_eq!(hash, 0xf8e7_140e_7180_c7b9, "got {hash:#018x}");
    }
}
