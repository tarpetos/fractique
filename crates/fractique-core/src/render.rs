use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::chaos::{Compiled, Walker};
use crate::deep;
use crate::math::{cos, sin};
use crate::palette;
use crate::rng::Rng;
use crate::scene::{Blend, Scene, SceneError};
use crate::tone;

/// Fixed, so that a render does not depend on how many cores the machine has.
pub const STREAMS: u64 = 64;
const CHUNK: u64 = 1 << 16;
const STREAM_DOMAIN: u64 = 0x1000;
const PLAN_DOMAIN: u64 = 0x2000;
const DEEP_DOMAIN: u64 = 0x3000;

pub const MAX_SIDE: u32 = 16_384;
pub const MAX_SUPERSAMPLE: u32 = 4;
pub const MAX_SAMPLES_PER_PIXEL: u32 = 100_000;
pub const MAX_CELLS: u64 = 1 << 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Draft,
    Mobile,
    Standard,
    High,
    Ultra,
}

impl Quality {
    pub const ALL: [(&'static str, Quality); 5] = [
        ("draft", Quality::Draft),
        ("mobile", Quality::Mobile),
        ("standard", Quality::Standard),
        ("high", Quality::High),
        ("ultra", Quality::Ultra),
    ];

    pub fn settings(self) -> (u32, u32) {
        match self {
            Quality::Draft => (8, 1),
            Quality::Mobile => (30, 1),
            Quality::Standard => (100, 1),
            Quality::High => (300, 2),
            Quality::Ultra => (1000, 2),
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|(n, _)| *n == name).map(|(_, q)| *q)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub samples_per_pixel: u32,
    pub supersample: u32,
}

impl RenderOptions {
    pub fn new(width: u32, height: u32, quality: Quality) -> Self {
        let (samples_per_pixel, supersample) = quality.settings();
        Self {
            width,
            height,
            samples_per_pixel,
            supersample,
        }
    }

    fn validate(&self) -> Result<(), RenderError> {
        let side = |v: u32| (1..=MAX_SIDE).contains(&v);
        if !side(self.width) || !side(self.height) {
            return Err(RenderError::Options(format!(
                "width and height must be between 1 and {MAX_SIDE}"
            )));
        }
        if !(1..=MAX_SUPERSAMPLE).contains(&self.supersample) {
            return Err(RenderError::Options(format!(
                "supersample must be between 1 and {MAX_SUPERSAMPLE}"
            )));
        }
        if !(1..=MAX_SAMPLES_PER_PIXEL).contains(&self.samples_per_pixel) {
            return Err(RenderError::Options(format!(
                "samples per pixel must be between 1 and {MAX_SAMPLES_PER_PIXEL}"
            )));
        }
        let cells = u64::from(self.width) * u64::from(self.height) * u64::from(self.supersample).pow(2);
        if cells > MAX_CELLS {
            return Err(RenderError::Options(
                "the image is too large for this supersample".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn hash(&self) -> u64 {
        self.rgba.iter().fold(0xCBF2_9CE4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01B3)
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    Scene(SceneError),
    Options(String),
    Cancelled,
}

impl core::fmt::Display for RenderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RenderError::Scene(e) => write!(f, "{e}"),
            RenderError::Options(e) => f.write_str(e),
            RenderError::Cancelled => f.write_str("the render was cancelled"),
        }
    }
}

impl std::error::Error for RenderError {}

#[derive(Default, Clone, Copy)]
pub struct Control<'a> {
    pub cancel: Option<&'a AtomicBool>,
    pub progress: Option<&'a (dyn Fn(f64) + Sync)>,
}

pub(crate) struct Histogram {
    pub(crate) width: usize,
    pub(crate) height: usize,
    counts: Vec<AtomicU32>,
    colors: Vec<AtomicU64>,
}

impl Histogram {
    fn new(width: usize, height: usize) -> Self {
        let cells = width * height;
        Self {
            width,
            height,
            counts: (0..cells).map(|_| AtomicU32::new(0)).collect(),
            colors: (0..cells * 3).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    // Integer sums do not depend on the order the threads add in: that is what keeps a parallel render exact.
    #[inline]
    fn add(&self, cell: usize, rgb: [u8; 3]) {
        self.counts[cell].fetch_add(1, Ordering::Relaxed);
        for (ch, value) in rgb.into_iter().enumerate() {
            self.colors[cell * 3 + ch].fetch_add(u64::from(value), Ordering::Relaxed);
        }
    }

    fn total(&self) -> u64 {
        self.counts
            .iter()
            .map(|count| u64::from(count.load(Ordering::Relaxed)))
            .sum()
    }

    #[inline]
    pub(crate) fn cell(&self, cell: usize) -> (u32, [u64; 3]) {
        let color = |ch: usize| self.colors[cell * 3 + ch].load(Ordering::Relaxed);
        (
            self.counts[cell].load(Ordering::Relaxed),
            [color(0), color(1), color(2)],
        )
    }
}

fn for_each_stream(run: impl Fn(u64) + Sync + Send) {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        (0..STREAMS).into_par_iter().for_each(run);
    }
    #[cfg(not(feature = "parallel"))]
    (0..STREAMS).for_each(run);
}

// The histogram of one layer and what turns its counts into densities. `plain` keeps to the plain chaos game.
fn gather(
    scene: &Scene,
    index: usize,
    options: &RenderOptions,
    control: &Control<'_>,
    done: &AtomicU64,
    plain: bool,
) -> Result<(Histogram, f64), RenderError> {
    let flame = &scene.layers[index].flame;
    let (width, height) = (options.width as usize, options.height as usize);
    let supersample = options.supersample as usize;
    let per_layer = u64::from(options.samples_per_pixel) * width as u64 * height as u64;
    let all_samples = (per_layer * scene.layers.len() as u64) as f64;
    let cancelled = || control.cancel.is_some_and(|flag| flag.load(Ordering::Relaxed));
    let histogram = Histogram::new(width * supersample, height * supersample);
    let compiled = Compiled::new(flame);
    let lut = palette::lut(&flame.palette);

    let (wide, high) = (histogram.width as f64, histogram.height as f64);
    let per_unit = flame.camera.zoom * wide.min(high) / 4.0;
    let angle = flame.camera.rotation.to_radians();
    let (rot_cos, rot_sin) = (cos(angle), sin(angle));
    let [center_x, center_y] = flame.camera.center;
    let up = if flame.camera.flip_y { -per_unit } else { per_unit };

    let plot = |x: f64, y: f64, color: f64| {
        let (dx, dy) = (x - center_x, y - center_y);
        let px = wide * 0.5 + (dx * rot_cos + dy * rot_sin) * per_unit;
        let py = high * 0.5 - (dy * rot_cos - dx * rot_sin) * up;
        if px >= 0.0 && px < wide && py >= 0.0 && py < high {
            let shade = ((color * 255.0 + 0.5) as usize).min(255);
            histogram.add(py as usize * histogram.width + px as usize, lut[shade]);
        }
    };
    let report = |count: u64, of: u64| {
        if let Some(progress) = control.progress {
            let share = (count as f64 * per_layer as f64 / of as f64) as u64;
            let so_far = done.fetch_add(share, Ordering::Relaxed) + share;
            progress((so_far as f64 / all_samples).min(1.0));
        }
    };

    let plan = if plain {
        None
    } else {
        deep::plan(
            &compiled,
            &flame.camera,
            wide,
            high,
            &|stream| Rng::derive(scene.seed, PLAN_DOMAIN + index as u64, stream),
            &cancelled,
        )
    };
    if cancelled() {
        return Err(RenderError::Cancelled);
    }
    let normalize = match &plan {
        None => {
            for_each_stream(|stream| {
                let mut rng = Rng::derive(scene.seed, STREAM_DOMAIN + index as u64, stream);
                let mut walker = Walker::new(&compiled, &mut rng);
                let mut left = per_layer / STREAMS + u64::from(stream < per_layer % STREAMS);
                while left > 0 && !cancelled() {
                    let count = left.min(CHUNK);
                    walker.run(&compiled, &mut rng, count, plot);
                    left -= count;
                    report(count, per_layer);
                }
            });
            let per_cell = f64::from(options.samples_per_pixel) / (supersample * supersample) as f64;
            flame.camera.zoom * flame.camera.zoom / per_cell
        }
        Some(plan) if plan.is_empty() => 0.0,
        Some(plan) => {
            for_each_stream(|stream| {
                let mut rng = Rng::derive(scene.seed, DEEP_DOMAIN + index as u64, stream);
                let mut tracer = deep::Tracer::new(&compiled, &mut rng);
                let mut chains = plan.chains(stream as usize, STREAMS as usize);
                let mut left = per_layer / STREAMS + u64::from(stream < per_layer % STREAMS);
                while left > 0 && !cancelled() {
                    let count = left.min(CHUNK);
                    for _ in 0..count {
                        let (x, y, color) = plan.point(&compiled, &mut chains, &mut tracer, &mut rng);
                        plot(x, y, color);
                    }
                    left -= count;
                    report(count, per_layer);
                }
            });
            let landed = histogram.total();
            if landed == 0 {
                0.0
            } else {
                plan.exposure * (wide * high) / landed as f64
            }
        }
    };
    if cancelled() {
        return Err(RenderError::Cancelled);
    }
    Ok((histogram, normalize))
}

pub fn render(scene: &Scene, options: &RenderOptions, control: &Control<'_>) -> Result<Image, RenderError> {
    scene.validate().map_err(RenderError::Scene)?;
    options.validate()?;

    let (width, height) = (options.width as usize, options.height as usize);
    let supersample = options.supersample as usize;
    let done = AtomicU64::new(0);

    let mut canvas: Vec<f32> = Vec::with_capacity(width * height * 3);
    for _ in 0..width * height {
        canvas.extend(scene.background.map(|c| c as f32));
    }

    for (index, layer) in scene.layers.iter().enumerate() {
        let flame = &layer.flame;
        let (histogram, normalize) = gather(scene, index, options, control, &done, false)?;

        let developed = tone::develop(&histogram, &flame.tone, supersample, normalize);
        composite(&mut canvas, &developed, layer.blend, layer.opacity as f32);
    }

    let mut rgba = Vec::with_capacity(width * height * 4);
    for (index, pixel) in canvas.chunks_exact(3).enumerate() {
        let threshold = BAYER[index / width % 8][index % width % 8];
        rgba.extend(pixel.iter().map(|c| (c.clamp(0.0, 1.0) * 255.0 + threshold) as u8));
        rgba.push(255);
    }
    Ok(Image {
        width: options.width,
        height: options.height,
        rgba,
    })
}

const fn bayer() -> [[f32; 8]; 8] {
    let mut out = [[0.0; 8]; 8];
    let mut y = 0;
    while y < 8 {
        let mut x = 0;
        while x < 8 {
            let (mut rank, mut bit) = (0u32, 0);
            while bit < 3 {
                let (xb, yb) = ((x >> bit) & 1, (y >> bit) & 1);
                rank |= ((xb ^ yb) << (5 - 2 * bit)) | (yb << (4 - 2 * bit));
                bit += 1;
            }
            out[y as usize][x as usize] = (rank as f32 + 0.5) / 64.0;
            x += 1;
        }
        y += 1;
    }
    out
}

const BAYER: [[f32; 8]; 8] = bayer();

fn composite(canvas: &mut [f32], layer: &[f32], blend: Blend, opacity: f32) {
    for (dst, src) in canvas.chunks_exact_mut(3).zip(layer.chunks_exact(4)) {
        let alpha = src[3] * opacity;
        for ch in 0..3 {
            let s = src[ch] * opacity;
            dst[ch] = match blend {
                Blend::Add => dst[ch] + s,
                Blend::Screen => 1.0 - (1.0 - dst[ch].min(1.0)) * (1.0 - s.min(1.0)),
                Blend::Over => s + dst[ch] * (1.0 - alpha),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A zoomed view drawn from particles shows what the plain chaos game shows there with far more points.
    #[test]
    fn a_deep_view_matches_the_plain_chaos_game() {
        let mut scene = crate::presets::get("aurora-spiral").unwrap();
        let camera = &mut scene.layers[0].flame.camera;
        (camera.center, camera.zoom) = ([-0.9, 0.56], 24.0);
        let side = 48;
        let counts = |samples: u32, plain: bool| {
            let options = RenderOptions {
                width: side,
                height: side,
                samples_per_pixel: samples,
                supersample: 1,
            };
            let (histogram, _) = gather(&scene, 0, &options, &Control::default(), &AtomicU64::new(0), plain).unwrap();
            let block = 6;
            let mut sums = vec![0f64; (side as usize / block).pow(2)];
            for cell in 0..(side * side) as usize {
                let (x, y) = (cell % side as usize / block, cell / side as usize / block);
                sums[y * (side as usize / block) + x] += f64::from(histogram.cell(cell).0);
            }
            let total: f64 = sums.iter().sum();
            assert!(total > 0.0);
            (sums.into_iter().map(|sum| sum / total).collect::<Vec<_>>(), total)
        };
        let (plain, landed) = counts(60_000, true);
        let (deep, _) = counts(2_000, false);
        assert!(
            landed < 0.125 * 60_000.0 * f64::from(side * side),
            "the view holds too much for a deep render"
        );
        let off: f64 = plain.iter().zip(&deep).map(|(a, b)| (a - b).abs()).sum();
        assert!(off < 0.05, "the deep view is {off} away from the plain one");
    }

    #[test]
    fn dither_thresholds_are_a_permutation() {
        let mut ranks: Vec<u32> = BAYER.iter().flatten().map(|t| (t * 64.0 - 0.5) as u32).collect();
        ranks.sort_unstable();
        assert_eq!(ranks, (0..64).collect::<Vec<_>>());
    }
}
