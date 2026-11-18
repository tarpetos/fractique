use super::{COLOR_DENSITY, COLOR_SHIFT, Coloring, EDGE, ESCAPE, FADE, ITERATIONS, TONE, candidate, limit, moved};
use crate::fractal::{Draw, Family, Field, Preset, Sample, Spec, stream};
use crate::rng::Rng;
use crate::scene::{Camera, Fractal};
use crate::fractal::Param;
use super::{judged, nudged, value};
use crate::fractal::random_palette;

const NAME: &str = "phoenix";
const ATTEMPTS: u64 = 24;
const WHOLE: Camera = Camera {
    center: [0.0, 0.0],
    zoom: 1.3,
    rotation: 0.0,
    flip_y: false,
};

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Escape,
    params: &[
        Param::whole("form", 1.0, 0.0, 1.0),
        Param::real("real", 0.566_67, -2.0, 2.0),
        Param::real("imaginary", 0.0, -2.0, 2.0),
        Param::real("feedback", -0.5, -1.5, 1.5),
        ITERATIONS,
        COLOR_DENSITY,
        COLOR_SHIFT,
        EDGE,
        FADE,
    ],
    tone: TONE,
    draw: Draw::Pixel(|params, camera| Box::new(Phoenix::new(params, camera))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "ushiki",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.45,
            params: &[("color_density", 1.4), ("color_shift", 0.3)],
        },
        Preset {
            name: "map",
            palette: "violet",
            center: [0.0, -0.3],
            zoom: 1.2,
            params: &[("form", 0.0), ("color_density", 1.3), ("color_shift", 0.5)],
        },
        Preset {
            name: "galaxy",
            palette: "orchid",
            center: [-0.44787, -0.34336],
            zoom: 3.8,
            params: &[("real", -0.197), ("imaginary", 0.4488), ("feedback", 0.31), ("iterations", 900.0), ("color_density", 0.43), ("color_shift", 1.41)],
        },
        Preset {
            name: "bubbles",
            palette: "ice",
            center: [0.42031, 0.24948],
            zoom: 4.8,
            params: &[("real", 0.4441), ("imaginary", 0.028), ("feedback", -0.28), ("iterations", 450.0), ("color_density", 0.85), ("color_shift", 1.74)],
        },
        Preset {
            name: "feathers",
            palette: "gold",
            center: [0.0626, -0.22253],
            zoom: 3.6,
            params: &[("real", -0.2165), ("imaginary", 0.4969), ("feedback", -0.46), ("iterations", 600.0), ("color_density", 0.31), ("color_shift", 0.42)],
        },
        Preset {
            name: "whorl",
            palette: "aurora",
            center: [-0.07532, -0.46833],
            zoom: 3.1,
            params: &[("real", -0.3821), ("imaginary", 0.2375), ("feedback", 0.25), ("iterations", 600.0), ("color_density", 0.58), ("color_shift", 1.42)],
        },
    ],
};

struct Phoenix {
    julia: bool,
    c: (f64, f64),
    feedback: f64,
    limit: u32,
    coloring: Coloring,
}

impl Phoenix {
    fn new(params: &[f64], camera: &Camera) -> Self {
        Self {
            julia: params[0] >= 0.5,
            c: (params[1], params[2]),
            feedback: params[3],
            limit: limit(params[4], camera.zoom),
            coloring: Coloring::new(params[5], params[6], params[7], params[8], camera.zoom),
        }
    }
}

impl Field for Phoenix {
    #[inline]
    fn sample(&self, right: f64, up: f64) -> Option<Sample> {
        // The picture stands on the real axis, as the phoenix is always shown: the plane is turned a quarter.
        let point = (up, -right);
        let ((cx, cy), (mut x, mut y), (mut dx, mut dy), grows) = if self.julia {
            (self.c, point, (1.0f64, 0.0f64), 0.0)
        } else {
            (point, (0.0f64, 0.0f64), (0.0f64, 0.0f64), 1.0)
        };
        let p = self.feedback;
        let (mut last_x, mut last_y, mut last_dx, mut last_dy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut old, mut lap, mut period) = ([f64::NAN; 4], 0u32, 8u32);
        for step in 0..self.limit {
            let (x2, y2) = (x * x, y * y);
            if x2 + y2 > ESCAPE {
                return Some(self.coloring.outside(step, x2 + y2, dx * dx + dy * dy));
            }
            let next = (
                2.0 * (x * dx - y * dy) + p * last_dx + grows,
                2.0 * (x * dy + y * dx) + p * last_dy,
            );
            (last_dx, last_dy) = (dx, dy);
            (dx, dy) = next;
            let next = (x2 - y2 + cx + p * last_x, 2.0 * x * y + cy + p * last_y);
            (last_x, last_y) = (x, y);
            (x, y) = next;
            if [x, y, last_x, last_y] == old {
                return None;
            }
            lap += 1;
            if lap == period {
                (old, lap, period) = ([x, y, last_x, last_y], 0, period.saturating_mul(2));
            }
        }
        None
    }
}

fn field(form: f64, c: (f64, f64), feedback: f64) -> impl Fn(&[f64; 5], &Camera) -> Box<dyn Field> {
    move |shared, camera| {
        Box::new(Phoenix::new(
            &[&[form, c.0, c.1, feedback][..], &shared[..]].concat(),
            camera,
        ))
    }
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let feedback = (rng.range(-0.75, 0.75) * 100.0).round() / 100.0;
        let map = field(0.0, (0.0, 0.0), feedback);
        let own = if rng.next_f64() < 0.3 {
            let (camera, colors, params) = candidate(&mut rng, &WHOLE, 6, &map);
            (camera, colors, params, vec![("form", 0.0), ("feedback", feedback)])
        } else {
            // The sets worth a picture belong to the points along the edge of the map of the first form.
            let steps = 2 + rng.below(4);
            let at = super::descend(&mut rng, steps, &WHOLE, &map).center;
            let c = (at[1], -at[0]);
            let c = ((c.0 * 1e4).round() / 1e4, (c.1 * 1e4).round() / 1e4);
            let mut camera = Camera {
                zoom: rng.range(1.2, 1.6),
                ..Camera::default()
            };
            if rng.next_f64() < 0.3 {
                camera = camera.looking_at(rng.range(-0.2, 0.2), rng.range(-0.2, 0.2), rng.range(1.5, 4.0));
            }
            let colors = random_palette(&mut rng);
            let own = vec![("form", 1.0), ("real", c.0), ("imaginary", c.1), ("feedback", feedback)];
            let params = judged(&mut rng, &camera, &field(1.0, c, feedback));
            (camera, colors, params, own)
        };
        let (camera, colors, params, own) = own;
        match params {
            Some(params) => return SPEC.fractal(colors, camera, &[&own[..], &params[..]].concat()),
            None => fallback.get_or_insert_with(|| SPEC.fractal(colors, camera, &own)),
        };
    }
    fallback.unwrap_or_else(|| SPEC.fractal("sunset", WHOLE, &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    if value(&SPEC, fractal, "form") < 0.5 {
        return moved(fractal, rng, strength);
    }
    let mut out = moved(fractal, rng, strength * 0.3);
    nudged(&SPEC, &mut out, "real", rng, 0.06 * strength);
    nudged(&SPEC, &mut out, "imaginary", rng, 0.06 * strength);
    nudged(&SPEC, &mut out, "feedback", rng, 0.06 * strength);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn both_forms_follow_the_same_map() {
        // z₁ = c and z₂ = c² + c from 0, as for the Mandelbrot set: with no feedback the first form is that set.
        let map = Phoenix::new(&[0.0, 0.0, 0.0, 0.0, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(map.sample(0.0, -1.0).is_none() && map.sample(0.0, 0.3).is_some());
        let julia = Phoenix::new(&[1.0, 0.566_67, 0.0, -0.5, 300.0, 1.0, 0.0, 0.0, 0.0], &Camera::default());
        assert!(julia.sample(0.65, 0.0).is_none() && julia.sample(0.0, 0.0).is_some());
        assert_eq!(julia.sample(0.3, 0.2).map(|sample| sample.at), julia.sample(-0.3, 0.2).map(|sample| sample.at));
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("phoenix-ushiki").unwrap());
        assert_eq!(hash, 0xdcb2_0f99_1db1_d51b, "got {hash:#018x}");
    }
}
