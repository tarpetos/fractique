use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::{Field, Sample};
use crate::math::{cos, pow, sin};
use crate::palette::{self, Lut};
use crate::render::RenderOptions;
use crate::scene::{Camera, Fractal};
use crate::tone;

// Neighbours that differ by less than this are taken for a smooth place, drawn from one sample a pixel.
const SMOOTH: f32 = 0.02;
const LARGEST_GRID: u32 = 8;

pub(crate) struct Frame {
    wide: f64,
    high: f64,
    per_unit: f64,
    up: f64,
    cos: f64,
    sin: f64,
    center: [f64; 2],
}

impl Frame {
    pub(crate) fn new(camera: &Camera, width: usize, height: usize) -> Self {
        let (wide, high) = (width as f64, height as f64);
        let per_unit = camera.zoom * wide.min(high) / 4.0;
        let angle = camera.rotation.to_radians();
        Self {
            wide,
            high,
            per_unit,
            up: if camera.flip_y { -per_unit } else { per_unit },
            cos: cos(angle),
            sin: sin(angle),
            center: camera.center,
        }
    }

    #[inline]
    pub(crate) fn world(&self, px: f64, py: f64) -> (f64, f64) {
        let across = (px - self.wide * 0.5) / self.per_unit;
        let along = (self.high * 0.5 - py) / self.up;
        (
            self.center[0] + across * self.cos - along * self.sin,
            self.center[1] + across * self.sin + along * self.cos,
        )
    }
}

// How many samples a side a pixel is drawn from where the picture is not smooth.
pub(crate) fn grid(options: &RenderOptions) -> usize {
    let base = match options.samples_per_pixel {
        0..16 => 1,
        16..64 => 2,
        64..600 => 3,
        _ => 4,
    };
    (base * options.supersample).min(LARGEST_GRID) as usize
}

// `curve` is 2.5 over the layer's gamma: at a flame's usual gamma the light is left as the kind gave it.
#[inline]
fn colored(lut: &Lut, curve: f64, sample: Option<Sample>) -> [f32; 4] {
    let Some(mut sample) = sample else {
        return [0.0; 4];
    };
    if curve != 1.0 {
        sample.light = pow(sample.light, curve);
    }
    let turn = sample.at * 0.5;
    let place = (1.0 - (2.0 * (turn - turn.floor()) - 1.0).abs()) * 255.0;
    let below = (place as usize).min(254);
    let share = (place - below as f64) as f32;
    let light = sample.light as f32;
    let (a, b) = (lut[below], lut[below + 1]);
    let channel = |ch: usize| {
        let (a, b) = (f32::from(a[ch]), f32::from(b[ch]));
        (a + (b - a) * share) * (1.0 / 255.0) * light
    };
    [channel(0), channel(1), channel(2), light.min(1.0)]
}

pub(crate) fn draw(
    field: &dyn Field,
    fractal: &Fractal,
    options: &RenderOptions,
    cancel: Option<&AtomicBool>,
    report: &(dyn Fn(f64) + Sync),
) -> Option<Vec<f32>> {
    let (width, height) = (options.width as usize, options.height as usize);
    let frame = Frame::new(&fractal.camera, width, height);
    let lut = palette::lut(&fractal.palette);
    let curve = 2.5 / fractal.tone.gamma;
    let side = grid(options);
    let cancelled = || cancel.is_some_and(|flag| flag.load(Ordering::Relaxed));
    let rows = AtomicUsize::new(0);
    // The second pass draws many samples only where the first found an edge: it is given the larger share.
    let first_share = if side == 1 { 1.0 } else { 0.25 };
    let count_up = |from: f64, share: f64| {
        let done = rows.fetch_add(1, Ordering::Relaxed) + 1;
        report(from + share * done as f64 / height as f64);
    };

    let mut centers = vec![0f32; width * height * 4];
    tone::for_each_row(&mut centers, width * 4, |y, row| {
        if cancelled() {
            return;
        }
        for x in 0..width {
            let (wx, wy) = frame.world(x as f64 + 0.5, y as f64 + 0.5);
            row[x * 4..x * 4 + 4].copy_from_slice(&colored(&lut, curve, field.sample(wx, wy)));
        }
        count_up(0.0, first_share);
    });
    if cancelled() {
        return None;
    }

    let mut out = centers.clone();
    if side > 1 {
        rows.store(0, Ordering::Relaxed);
        let step = 1.0 / side as f64;
        let share = (step * step) as f32;
        let at = |x: usize, y: usize| &centers[(y * width + x) * 4..(y * width + x) * 4 + 4];
        tone::for_each_row(&mut out, width * 4, |y, row| {
            if cancelled() {
                return;
            }
            let (y_from, y_to) = (y.saturating_sub(1), (y + 1).min(height - 1));
            for x in 0..width {
                let (x_from, x_to) = (x.saturating_sub(1), (x + 1).min(width - 1));
                let own = at(x, y);
                let smooth = (y_from..=y_to).all(|ny| {
                    (x_from..=x_to).all(|nx| at(nx, ny).iter().zip(own).all(|(a, b)| (a - b).abs() < SMOOTH))
                });
                if smooth {
                    continue;
                }
                let mut sum = [0f32; 4];
                for sy in 0..side {
                    for sx in 0..side {
                        let (wx, wy) =
                            frame.world(x as f64 + (sx as f64 + 0.5) * step, y as f64 + (sy as f64 + 0.5) * step);
                        let color = colored(&lut, curve, field.sample(wx, wy));
                        for ch in 0..4 {
                            sum[ch] += color[ch];
                        }
                    }
                }
                for ch in 0..4 {
                    row[x * 4 + ch] = sum[ch] * share;
                }
            }
            count_up(first_share, 1.0 - first_share);
        });
        if cancelled() {
            return None;
        }
    }

    let brightness = fractal.tone.brightness as f32;
    if brightness != 1.0 {
        tone::for_each_row(&mut out, width * 4, |_, row| {
            for pixel in row.chunks_exact_mut(4) {
                for value in &mut pixel[..3] {
                    *value *= brightness;
                }
            }
        });
    }
    let sigma = fractal.tone.glow_radius * width.min(height) as f64;
    if fractal.tone.glow_strength > 0.0 && sigma >= 0.5 {
        tone::glow(&mut out, width, height, sigma, fractal.tone.glow_strength as f32);
    }
    Some(out)
}

pub(crate) const PROBE: usize = 48;

// The samples of a square grid over the middle of the view: what a randomizer judges a view by.
pub(crate) fn probe(field: &dyn Field, camera: &Camera) -> Vec<Option<Sample>> {
    let frame = Frame::new(camera, PROBE, PROBE);
    (0..PROBE * PROBE)
        .map(|cell| {
            let (wx, wy) = frame.world((cell % PROBE) as f64 + 0.5, (cell / PROBE) as f64 + 0.5);
            field.sample(wx, wy)
        })
        .collect()
}
