use crate::chaos::{Compiled, Walker};
use crate::math::{TAU, cos, sin};
use crate::palette;
use crate::rng::Rng;
use crate::scene::{Blend, Camera, Flame, Layer, SCENE_VERSION, Scene, Tone, Transform};
use crate::variation::{Kind, Variation};

const RANDOM_DOMAIN: u64 = 0x2000;
const MUTATE_DOMAIN: u64 = 0x3000;
const PROBE_DOMAIN: u64 = 0x4000;
const PROBE_POINTS: u64 = 30_000;
const GRID: usize = 48;
const ATTEMPTS: u64 = 40;

const CHOICES: [(Kind, u32); 27] = [
    (Kind::Linear, 6),
    (Kind::Spherical, 5),
    (Kind::Julian, 5),
    (Kind::Curl, 4),
    (Kind::Sinusoidal, 3),
    (Kind::Swirl, 3),
    (Kind::Julia, 3),
    (Kind::Bubble, 3),
    (Kind::Polar, 2),
    (Kind::Disc, 2),
    (Kind::Hyperbolic, 2),
    (Kind::Eyefish, 2),
    (Kind::Horseshoe, 2),
    (Kind::Spiral, 2),
    (Kind::Handkerchief, 2),
    (Kind::Heart, 1),
    (Kind::Diamond, 1),
    (Kind::Cylinder, 1),
    (Kind::Cosine, 1),
    (Kind::Exponential, 1),
    (Kind::Fisheye, 1),
    (Kind::Bent, 1),
    (Kind::Power, 1),
    (Kind::Ex, 1),
    (Kind::Tangent, 1),
    (Kind::Cross, 1),
    (Kind::GaussianBlur, 1),
];

fn pick_kind(rng: &mut Rng) -> Kind {
    let total: u32 = CHOICES.iter().map(|(_, w)| w).sum();
    let mut roll = rng.below(u64::from(total)) as u32;
    for (kind, weight) in CHOICES {
        if roll < weight {
            return kind;
        }
        roll -= weight;
    }
    Kind::Linear
}

fn random_variation(rng: &mut Rng, weight: f64) -> Variation {
    let kind = pick_kind(rng);
    let params = match kind {
        Kind::Julian => {
            let power = (2 + rng.below(5)) as f64 * if rng.next_bool() { 1.0 } else { -1.0 };
            vec![power, [1.0, -1.0, 0.5][rng.below(3) as usize]]
        }
        Kind::Curl => vec![rng.range(-0.6, 0.6), rng.range(-0.4, 0.4)],
        _ => Vec::new(),
    };
    Variation { kind, weight, params }
}

fn random_affine(rng: &mut Rng) -> [f64; 6] {
    let angle = rng.range(0.0, TAU);
    let skew = angle + rng.range(-0.4, 0.4);
    let scale = rng.range(0.35, 0.95);
    let (sx, sy) = (scale * rng.range(0.8, 1.2), scale * rng.range(0.8, 1.2));
    [
        sx * cos(angle),
        -sy * sin(skew),
        rng.range(-0.9, 0.9),
        sx * sin(angle),
        sy * cos(skew),
        rng.range(-0.9, 0.9),
    ]
}

fn random_flame(rng: &mut Rng) -> Flame {
    let count = 2 + rng.below(3) as usize;
    let transforms = (0..count)
        .map(|i| {
            let main_weight = rng.range(0.6, 1.0);
            let mut variations = vec![random_variation(rng, main_weight)];
            if rng.next_f64() < 0.4 {
                let extra_weight = rng.range(0.1, 0.5);
                variations.push(random_variation(rng, extra_weight));
            }
            Transform {
                weight: rng.range(0.4, 1.0),
                color: (i as f64 / (count - 1) as f64 + rng.range(-0.1, 0.1)).clamp(0.0, 1.0),
                color_speed: rng.range(0.3, 0.7),
                opacity: 1.0,
                affine: random_affine(rng),
                post: None,
                variations,
            }
        })
        .collect();
    let symmetry = if rng.next_f64() < 0.55 {
        1
    } else {
        [2, 3, 4, 5, 6, -1, -2, -3, -4][rng.below(9) as usize]
    };
    let name = palette::NAMES[rng.below(palette::NAMES.len() as u64) as usize];
    Flame {
        camera: Camera::default(),
        tone: Tone {
            brightness: rng.range(0.9, 1.3),
            gamma: rng.range(2.2, 3.2),
            vibrancy: 1.0,
            highlight: rng.range(0.0, 0.4),
            glow_radius: 0.006,
            glow_strength: 0.5,
            smoothing: 1.0,
        },
        palette: palette::named(name).unwrap_or_else(|| palette::named("aurora").unwrap()),
        symmetry,
        transforms,
        final_transform: None,
    }
}

pub fn fit_camera(flame: &Flame) -> Option<(Camera, bool)> {
    let compiled = Compiled::new(flame);
    let mut rng = Rng::derive(0, PROBE_DOMAIN, 0);
    let mut walker = Walker::new(&compiled, &mut rng);
    let mut points = Vec::with_capacity(PROBE_POINTS as usize);
    walker.run(&compiled, &mut rng, PROBE_POINTS, |x, y, _| points.push((x, y)));
    if points.len() < 1000 {
        return None;
    }

    let span = |pick: fn(&(f64, f64)) -> f64| {
        let mut values: Vec<f64> = points.iter().map(pick).collect();
        values.sort_by(f64::total_cmp);
        (values[values.len() / 50], values[values.len() - 1 - values.len() / 50])
    };
    let (x_lo, x_hi) = span(|p| p.0);
    let (y_lo, y_hi) = span(|p| p.1);
    let half = ((x_hi - x_lo).max(y_hi - y_lo)) * 0.5 * 1.25;
    if !(half > 1e-3 && half < 1e6) {
        return None;
    }
    let center = [(x_lo + x_hi) * 0.5, (y_lo + y_hi) * 0.5];

    let mut seen = [false; GRID * GRID];
    let mut inside = 0usize;
    for (x, y) in &points {
        let gx = ((x - center[0]) / half * 0.5 + 0.5) * GRID as f64;
        let gy = ((y - center[1]) / half * 0.5 + 0.5) * GRID as f64;
        if gx >= 0.0 && gx < GRID as f64 && gy >= 0.0 && gy < GRID as f64 {
            seen[gy as usize * GRID + gx as usize] = true;
            inside += 1;
        }
    }
    let covered = seen.iter().filter(|s| **s).count() as f64 / (GRID * GRID) as f64;
    let good = (0.12..=0.8).contains(&covered) && inside * 10 >= points.len() * 7;
    Some((
        Camera {
            center,
            zoom: 2.0 / half,
            rotation: 0.0,
            flip_y: flame.camera.flip_y,
        },
        good,
    ))
}

fn framed_flame(seed: u64, salt: u64) -> Flame {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = Rng::derive(seed, RANDOM_DOMAIN + salt, attempt);
        let mut flame = random_flame(&mut rng);
        if let Some((camera, good)) = fit_camera(&flame) {
            flame.camera = camera;
            if good {
                return flame;
            }
            fallback.get_or_insert(flame);
        }
    }
    fallback.unwrap_or_else(|| random_flame(&mut Rng::derive(seed, RANDOM_DOMAIN + salt, ATTEMPTS)))
}

pub fn random_scene(seed: u64) -> Scene {
    let mut rng = Rng::derive(seed, RANDOM_DOMAIN, u64::MAX);
    let mut layers = vec![Layer {
        name: "base".into(),
        blend: Blend::Add,
        opacity: 1.0,
        flame: framed_flame(seed, 0),
    }];
    if rng.next_f64() < 0.35 {
        layers.push(Layer {
            name: "veil".into(),
            blend: Blend::Add,
            opacity: rng.range(0.4, 0.75),
            flame: framed_flame(seed, 1),
        });
    }
    Scene {
        version: SCENE_VERSION,
        name: format!("random-{seed}"),
        seed,
        background: [0.008, 0.01, 0.03],
        sheets: None,
        layers,
    }
}

pub fn mutate(scene: &Scene, seed: u64, strength: f64) -> Scene {
    let strength = strength.clamp(0.0, 1.0);
    let mut out = scene.clone();
    for (index, layer) in out.layers.iter_mut().enumerate() {
        let mut rng = Rng::derive(seed, MUTATE_DOMAIN, index as u64);
        for transform in &mut layer.flame.transforms {
            for value in &mut transform.affine {
                *value += rng.range(-0.35, 0.35) * strength;
            }
            transform.color = (transform.color + rng.range(-0.2, 0.2) * strength).clamp(0.0, 1.0);
            for variation in &mut transform.variations {
                variation.weight *= 1.0 + rng.range(-0.3, 0.3) * strength;
            }
        }
    }
    if !out.name.is_empty() {
        out.name = format!("{}-mutated-{seed}", scene.name);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_always_gives_the_same_scene() {
        for seed in [0, 1, 99, u64::MAX] {
            let scene = random_scene(seed);
            assert_eq!(scene, random_scene(seed));
            scene.validate().unwrap();
        }
        assert_ne!(random_scene(1), random_scene(2));
    }

    #[test]
    fn mutation_is_repeatable_and_valid() {
        let scene = random_scene(5);
        let mutated = mutate(&scene, 9, 0.5);
        assert_eq!(mutated, mutate(&scene, 9, 0.5));
        assert_ne!(mutated, scene);
        mutated.validate().unwrap();
        assert_eq!(mutate(&scene, 9, 0.0).layers, scene.layers);
    }
}
