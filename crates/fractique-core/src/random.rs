use crate::chaos::{Compiled, Walker};
use crate::fractal;
use crate::math::{TAU, cos, sin};
use crate::palette;
use crate::rng::Rng;
use crate::scene::{Blend, Camera, Content, Flame, Layer, SCENE_VERSION, Scene, Tone, Transform};
use crate::variation::{Kind, Variation};

const RANDOM_DOMAIN: u64 = 0x2000;
const SECOND_DOMAIN: u64 = 0x5000;
const MUTATE_DOMAIN: u64 = 0x3000;
const PROBE_DOMAIN: u64 = 0x4000;
pub(crate) const PROBE_POINTS: u64 = 30_000;
pub(crate) const GRID: usize = 48;
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

const SECOND_CHOICES: [(Kind, u32); 49] = [
    (Kind::Linear, 6),
    (Kind::Spherical, 5),
    (Kind::Julian, 5),
    (Kind::Curl, 4),
    (Kind::Juliascope, 4),
    (Kind::Sinusoidal, 3),
    (Kind::Swirl, 3),
    (Kind::Julia, 3),
    (Kind::Bubble, 3),
    (Kind::Waves, 3),
    (Kind::Blob, 3),
    (Kind::Pdj, 3),
    (Kind::Ngon, 3),
    (Kind::Polar, 2),
    (Kind::Disc, 2),
    (Kind::Hyperbolic, 2),
    (Kind::Eyefish, 2),
    (Kind::Horseshoe, 2),
    (Kind::Spiral, 2),
    (Kind::Handkerchief, 2),
    (Kind::Popcorn, 2),
    (Kind::Rings, 2),
    (Kind::Fan, 2),
    (Kind::Fan2, 2),
    (Kind::Rings2, 2),
    (Kind::Perspective, 2),
    (Kind::RadialBlur, 2),
    (Kind::Rectangles, 2),
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
    (Kind::Blur, 1),
    (Kind::GaussianBlur, 1),
    (Kind::Noise, 1),
    (Kind::Pie, 1),
    (Kind::Arch, 1),
    (Kind::Square, 1),
    (Kind::Rays, 1),
    (Kind::Blade, 1),
    (Kind::Secant2, 1),
    (Kind::Twintrian, 1),
];

fn pick_kind(rng: &mut Rng, choices: &[(Kind, u32)]) -> Kind {
    let total: u32 = choices.iter().map(|(_, w)| w).sum();
    let mut roll = rng.below(u64::from(total)) as u32;
    for &(kind, weight) in choices {
        if roll < weight {
            return kind;
        }
        roll -= weight;
    }
    Kind::Linear
}

fn random_variation(rng: &mut Rng, weight: f64) -> Variation {
    let kind = pick_kind(rng, &CHOICES);
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

fn ignores_its_point(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Blur | Kind::GaussianBlur | Kind::Square | Kind::Pie | Kind::Arch
    )
}

fn smears(kind: Kind) -> bool {
    matches!(kind, Kind::Noise | Kind::RadialBlur)
}

fn either_sign(rng: &mut Rng, lo: f64, hi: f64) -> f64 {
    rng.range(lo, hi) * if rng.next_bool() { 1.0 } else { -1.0 }
}

fn second_params(kind: Kind, rng: &mut Rng) -> Vec<f64> {
    match kind {
        Kind::Julian | Kind::Juliascope => {
            let power = (2 + rng.below(5)) as f64 * if rng.next_bool() { 1.0 } else { -1.0 };
            vec![power, [1.0, -1.0, 0.5][rng.below(3) as usize]]
        }
        Kind::Curl => vec![rng.range(-0.6, 0.6), rng.range(-0.4, 0.4)],
        Kind::Blob => vec![rng.range(0.2, 0.7), rng.range(0.9, 1.4), (2 + rng.below(7)) as f64],
        Kind::Pdj => (0..4).map(|_| either_sign(rng, 0.8, 3.0)).collect(),
        Kind::Fan2 => vec![rng.range(0.2, 0.9), rng.range(-1.0, 1.0)],
        Kind::Rings2 => vec![rng.range(0.2, 0.9)],
        Kind::Perspective => vec![rng.range(0.1, 0.8), rng.range(1.5, 3.5)],
        Kind::RadialBlur => vec![rng.range(0.0, 1.0)],
        Kind::Pie => vec![(3 + rng.below(7)) as f64, rng.range(0.0, TAU), rng.range(0.2, 0.8)],
        Kind::Ngon => vec![
            (3 + rng.below(6)) as f64,
            rng.range(1.5, 3.5),
            rng.range(0.5, 1.5),
            rng.range(0.5, 1.5),
        ],
        _ => Vec::new(),
    }
}

fn second_variation(rng: &mut Rng, weight: f64, refused: fn(Kind) -> bool) -> Variation {
    // `rectangles` tiles a flame into squares: it is drawn again. It keeps its place among the choices so that
    // the seeds that never drew it keep their scenes.
    let mut kind = pick_kind(rng, &SECOND_CHOICES);
    while refused(kind) || kind == Kind::Rectangles {
        kind = pick_kind(rng, &SECOND_CHOICES);
    }
    Variation {
        kind,
        weight,
        params: second_params(kind, rng),
    }
}

fn second_flame(rng: &mut Rng) -> Flame {
    let count = 2 + rng.below(3) as usize;
    let mut placed = false;
    let transforms = (0..count)
        .map(|i| {
            let main_weight = rng.range(0.6, 1.0);
            let main = second_variation(
                rng,
                main_weight,
                if placed || count < 3 {
                    |kind| kind == Kind::Noise || ignores_its_point(kind)
                } else {
                    |_| false
                },
            );
            let post = ignores_its_point(main.kind).then(|| small_affine(rng));
            placed |= post.is_some();
            let mut variations = vec![main];
            if post.is_none() && rng.next_f64() < 0.4 {
                let extra_weight = rng.range(0.1, 0.5);
                variations.push(second_variation(rng, extra_weight, |kind| {
                    smears(kind) || ignores_its_point(kind)
                }));
            }
            Transform {
                weight: rng.range(0.4, 1.0),
                color: (i as f64 / (count - 1) as f64 + rng.range(-0.1, 0.1)).clamp(0.0, 1.0),
                color_speed: rng.range(0.3, 0.7),
                opacity: 1.0,
                affine: random_affine(rng),
                post,
                variations,
            }
        })
        .collect();
    finished(rng, transforms)
}

fn small_affine(rng: &mut Rng) -> [f64; 6] {
    let angle = rng.range(0.0, TAU);
    let scale = rng.range(0.15, 0.5);
    [
        scale * cos(angle),
        -scale * sin(angle),
        rng.range(-0.8, 0.8),
        scale * sin(angle),
        scale * cos(angle),
        rng.range(-0.8, 0.8),
    ]
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
    finished(rng, transforms)
}

fn finished(rng: &mut Rng, transforms: Vec<Transform>) -> Flame {
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

pub(crate) struct Probe {
    pub(crate) camera: Camera,
    pub(crate) cells: Vec<u32>,
    pub(crate) inside: usize,
    pub(crate) points: usize,
}

impl Probe {
    pub(crate) fn framed(&self) -> bool {
        let covered = self.cells.iter().filter(|hits| **hits > 0).count() as f64 / (GRID * GRID) as f64;
        (0.12..=0.8).contains(&covered) && self.inside * 10 >= self.points * 7
    }

    pub(crate) fn contrast(&self) -> usize {
        let step = |a: usize, b: usize| self.cells[a].abs_diff(self.cells[b]) as usize;
        let mut sum = 0;
        for y in 0..GRID {
            for x in 0..GRID {
                let at = y * GRID + x;
                if x + 1 < GRID {
                    sum += step(at, at + 1);
                }
                if y + 1 < GRID {
                    sum += step(at, at + GRID);
                }
            }
        }
        sum / 2
    }
}

type Verdict = fn(&Probe) -> (bool, f64);

fn first_verdict(probe: &Probe) -> (bool, f64) {
    (probe.framed(), 1.0)
}

// The limits were tuned on the pictures of seeds 1 to 144. They refuse specks, a bright dot in a wide haze and
// an even fog, which the first generator lets through, and move the camera closer to a small core in a haze.
fn second_verdict(probe: &Probe) -> (bool, f64) {
    let mut sorted = probe.cells.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let holding = |percent: usize| {
        let mut sum = 0;
        sorted
            .iter()
            .take_while(|hits| {
                let short = sum * 100 < probe.inside * percent;
                sum += **hits as usize;
                short
            })
            .count()
    };
    let covered = sorted.iter().filter(|hits| **hits > 0).count();
    let solid = sorted.iter().filter(|hits| **hits >= 4).count();
    let (half, most) = (holding(50), holding(90));
    let good = probe.framed()
        && covered * 100 >= GRID * GRID * 16
        && (6..=400).contains(&half)
        && most >= 130
        && (half >= 20 || solid * 100 >= covered * 36)
        && probe.contrast() * 100 >= probe.inside * 37;
    let closer = half < 45 && covered * 100 > GRID * GRID * 52;
    (good, if closer { 1.6 } else { 1.0 })
}

pub fn fit_camera(flame: &Flame) -> Option<(Camera, bool)> {
    let probe = probe(flame)?;
    let good = probe.framed();
    Some((probe.camera, good))
}

fn probe(flame: &Flame) -> Option<Probe> {
    let compiled = Compiled::new(flame);
    let mut rng = Rng::derive(0, PROBE_DOMAIN, 0);
    let mut walker = Walker::new(&compiled, &mut rng);
    let mut points = Vec::with_capacity(PROBE_POINTS as usize);
    walker.run(&compiled, &mut rng, PROBE_POINTS, |x, y, _| points.push((x, y)));
    probe_of(points, flame.camera.flip_y)
}

pub(crate) fn probe_of(points: Vec<(f64, f64)>, flip_y: bool) -> Option<Probe> {
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

    let mut cells = vec![0u32; GRID * GRID];
    let mut inside = 0usize;
    for (x, y) in &points {
        let gx = ((x - center[0]) / half * 0.5 + 0.5) * GRID as f64;
        let gy = ((y - center[1]) / half * 0.5 + 0.5) * GRID as f64;
        if gx >= 0.0 && gx < GRID as f64 && gy >= 0.0 && gy < GRID as f64 {
            cells[gy as usize * GRID + gx as usize] += 1;
            inside += 1;
        }
    }
    Some(Probe {
        camera: Camera {
            center,
            zoom: 2.0 / half,
            rotation: 0.0,
            flip_y,
        },
        cells,
        inside,
        points: points.len(),
    })
}

fn framed_flame(seed: u64, domain: u64, make: fn(&mut Rng) -> Flame, verdict: Verdict) -> Flame {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = Rng::derive(seed, domain, attempt);
        let mut flame = make(&mut rng);
        if let Some(probe) = probe(&flame) {
            let (good, closer) = verdict(&probe);
            flame.camera = probe.camera;
            flame.camera.zoom *= closer;
            if good {
                return flame;
            }
            fallback.get_or_insert(flame);
        }
    }
    fallback.unwrap_or_else(|| make(&mut Rng::derive(seed, domain, ATTEMPTS)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generator {
    First,
    Second,
}

impl Generator {
    pub const LATEST: Generator = Generator::Second;

    pub fn from_number(number: u64) -> Option<Self> {
        match number {
            1 => Some(Generator::First),
            2 => Some(Generator::Second),
            _ => None,
        }
    }

    pub fn number(self) -> u64 {
        match self {
            Generator::First => 1,
            Generator::Second => 2,
        }
    }
}

pub fn random_scene(seed: u64) -> Scene {
    random_scene_of(Generator::First, seed)
}

pub fn random_scene_of(generator: Generator, seed: u64) -> Scene {
    type Make = fn(&mut Rng) -> Flame;
    let (domain, make, verdict, name): (u64, Make, Verdict, String) = match generator {
        Generator::First => (RANDOM_DOMAIN, random_flame, first_verdict, format!("random-{seed}")),
        Generator::Second => (SECOND_DOMAIN, second_flame, second_verdict, format!("random2-{seed}")),
    };
    let mut rng = Rng::derive(seed, domain, u64::MAX);
    let mut layers = vec![Layer {
        name: "base".into(),
        blend: Blend::Add,
        opacity: 1.0,
        content: Content::Flame(framed_flame(seed, domain, make, verdict)),
    }];
    if rng.next_f64() < 0.35 {
        layers.push(Layer {
            name: "veil".into(),
            blend: Blend::Add,
            opacity: rng.range(0.4, 0.75),
            content: Content::Flame(framed_flame(seed, domain + 1, make, verdict)),
        });
    }
    Scene {
        version: SCENE_VERSION,
        name,
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
        let flame = match &mut layer.content {
            Content::Flame(flame) => flame,
            Content::Fractal(shown) => {
                if let Some(spec) = fractal::spec(&shown.kind) {
                    *shown = (spec.mutate)(shown, &mut rng, strength);
                }
                continue;
            }
        };
        for transform in &mut flame.transforms {
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

    fn fingerprint(scene: &Scene) -> u64 {
        scene.to_json().bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
        })
    }

    const FIRST: [(u64, u64); 7] = [
        (0, 0xeee6_d5eb_b777_4c8c),
        (1, 0xb1c1_d275_82c8_2c38),
        (7, 0xcda6_5b46_ee1d_6746),
        (42, 0x0922_d17f_3f90_ca90),
        (99, 0xe144_d5f4_134b_a230),
        (2024, 0x9021_e08e_7d25_a103),
        (u64::MAX, 0x74e8_9f8f_7bad_0668),
    ];

    #[test]
    fn the_first_generator_keeps_its_scenes() {
        let wrong: Vec<String> = FIRST
            .iter()
            .map(|(seed, _)| (*seed, fingerprint(&random_scene(*seed))))
            .zip(FIRST)
            .filter(|((_, got), (_, expected))| got != expected)
            .map(|((seed, got), _)| format!("({seed}, {got:#018x})"))
            .collect();
        assert!(wrong.is_empty(), "got {}", wrong.join(", "));
    }

    const SECOND: [(u64, u64); 7] = [
        (0, 0x6288_0f16_c9c8_598a),
        (1, 0xad33_53b7_bbe7_b4cf),
        (7, 0x2446_6684_142f_c23e),
        (42, 0x1bd4_0d33_5b23_cb91),
        (99, 0xe988_9ca8_caf2_5d98),
        (2024, 0xf8ae_8bb7_75d8_06d4),
        (u64::MAX, 0xa8b5_6514_bfb6_6ec6),
    ];

    #[test]
    fn the_second_generator_keeps_its_scenes() {
        let wrong: Vec<String> = SECOND
            .iter()
            .map(|(seed, _)| (*seed, fingerprint(&random_scene_of(Generator::Second, *seed))))
            .zip(SECOND)
            .filter(|((_, got), (_, expected))| got != expected)
            .map(|((seed, got), _)| format!("({seed}, {got:#018x})"))
            .collect();
        assert!(wrong.is_empty(), "got {}", wrong.join(", "));
    }

    fn kinds_of(generator: Generator, seeds: u64) -> Vec<Kind> {
        let mut kinds = Vec::new();
        for seed in 0..seeds {
            let scene = random_scene_of(generator, seed);
            scene.validate().unwrap();
            for layer in &scene.layers {
                for variation in layer
                    .flame()
                    .into_iter()
                    .flat_map(|flame| &flame.transforms)
                    .flat_map(|t| &t.variations)
                {
                    if !kinds.contains(&variation.kind) {
                        kinds.push(variation.kind);
                    }
                }
            }
        }
        kinds
    }

    #[test]
    fn the_second_generator_uses_every_variation_but_rectangles_and_the_first_its_own() {
        let second = kinds_of(Generator::Second, 400);
        for kind in Kind::ALL {
            assert_eq!(second.contains(&kind), kind != Kind::Rectangles, "{kind:?}");
        }
        let first = kinds_of(Generator::First, 200);
        assert!(
            first
                .iter()
                .all(|kind| CHOICES.iter().any(|(listed, _)| listed == kind))
        );
    }

    #[test]
    fn the_generators_are_numbered() {
        assert_eq!(random_scene_of(Generator::First, 5), random_scene(5));
        assert_ne!(random_scene_of(Generator::Second, 5).layers, random_scene(5).layers);
        for generator in [Generator::First, Generator::Second] {
            assert_eq!(Generator::from_number(generator.number()), Some(generator));
        }
        assert_eq!(Generator::from_number(0), None);
        assert_eq!(Generator::from_number(3), None);
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
