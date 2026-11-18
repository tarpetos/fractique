use crate::math::{exp, ln_1p, pow};
use crate::render::Histogram;
use crate::scene::Tone;

// A cell with about 3000 times the average density reaches full brightness at brightness 1.
const LOG_SCALE: f64 = 1.0 / 8.0;

fn for_each_row(buffer: &mut [f32], row_len: usize, fill: impl Fn(usize, &mut [f32]) + Sync) {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        buffer
            .par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(y, row)| fill(y, row));
    }
    #[cfg(not(feature = "parallel"))]
    buffer.chunks_mut(row_len).enumerate().for_each(|(y, row)| fill(y, row));
}

// Widest density-estimation kernel, in histogram cells: the cost of a render grows with its square.
const MAX_SPREAD: f64 = 9.0;
const SPREAD_SHARE: f64 = 0.004;
// How fast the kernel narrows as a cell collects hits (flam3's estimator curve).
const SPREAD_CURVE: f64 = 0.4;

struct Kernel {
    reach: usize,
    weights: Vec<f32>,
}

fn kernels(levels: usize) -> Vec<Kernel> {
    (0..=levels)
        .map(|level| {
            let width = level as f64 * 0.5;
            let reach = width.ceil() as usize;
            let sigma = (width * 0.5).max(0.3);
            let side = 2 * reach + 1;
            let mut weights: Vec<f32> = (0..side * side)
                .map(|i| {
                    let (dx, dy) = ((i % side) as f64 - reach as f64, (i / side) as f64 - reach as f64);
                    exp(-(dx * dx + dy * dy) / (2.0 * sigma * sigma)) as f32
                })
                .collect();
            let total: f32 = weights.iter().sum();
            weights.iter_mut().for_each(|w| *w /= total);
            Kernel { reach, weights }
        })
        .collect()
}

pub(crate) fn estimate(histogram: &Histogram, smoothing: f64) -> Vec<f32> {
    let (width, height) = (histogram.width, histogram.height);
    let spread = (SPREAD_SHARE * smoothing * width.min(height) as f64).min(MAX_SPREAD);
    let top = (spread * 2.0).round() as usize;
    let mut out = vec![0f32; width * height * 4];
    if top == 0 {
        for_each_row(&mut out, width * 4, |y, row| {
            for x in 0..width {
                let (count, colors) = histogram.cell(y * width + x);
                row[x * 4] = count as f32;
                for ch in 0..3 {
                    row[x * 4 + 1 + ch] = colors[ch] as f32;
                }
            }
        });
        return out;
    }

    let table = kernels(top);
    let by_count: Vec<u8> = (0..4096u32)
        .map(|count| {
            let wide = spread / pow(f64::from(count.max(1)), SPREAD_CURVE);
            if wide < 0.4 {
                0
            } else {
                ((wide * 2.0).round() as usize).min(top) as u8
            }
        })
        .collect();
    let level_of = |count: u32| by_count.get(count as usize).copied().unwrap_or(0);

    let mut levels = vec![0f32; width * height];
    for_each_row(&mut levels, width, |y, row| {
        for (x, level) in row.iter_mut().enumerate() {
            let count = histogram.cell(y * width + x).0;
            *level = if count == 0 { -1.0 } else { f32::from(level_of(count)) };
        }
    });

    let reach = table[top].reach;
    for_each_row(&mut out, width * 4, |y, row| {
        let (y_from, y_to) = (y.saturating_sub(reach), (y + reach).min(height - 1));
        for x in 0..width {
            let (x_from, x_to) = (x.saturating_sub(reach), (x + reach).min(width - 1));
            let mut sum = [0f32; 4];
            for sy in y_from..=y_to {
                for sx in x_from..=x_to {
                    let level = levels[sy * width + sx];
                    if level < 0.0 {
                        continue;
                    }
                    let kernel = &table[level as usize];
                    if sx.abs_diff(x) > kernel.reach || sy.abs_diff(y) > kernel.reach {
                        continue;
                    }
                    let side = 2 * kernel.reach + 1;
                    let k = kernel.weights[offset(y, sy, kernel.reach) * side + offset(x, sx, kernel.reach)];
                    let (count, colors) = histogram.cell(sy * width + sx);
                    sum[0] += count as f32 * k;
                    for ch in 0..3 {
                        sum[1 + ch] += colors[ch] as f32 * k;
                    }
                }
            }
            row[x * 4..x * 4 + 4].copy_from_slice(&sum);
        }
    });
    out
}

#[inline]
fn offset(center: usize, at: usize, reach: usize) -> usize {
    at + reach - center
}

// The average color of every pixel and its brightness before `Tone` shapes it: what a preview starts from.
pub(crate) fn levels(cells: &[f32], across: usize, supersample: usize, normalize: f64) -> Vec<f32> {
    let width = across / supersample;
    let height = cells.len() / 4 / across / supersample;
    let share = 1.0 / (supersample * supersample) as f64;
    let mut out = vec![0f32; width * height * 4];
    for_each_row(&mut out, width * 4, |y, row| {
        for x in 0..width {
            let mut sum = [0f64; 5];
            for sy in 0..supersample {
                for sx in 0..supersample {
                    let cell = ((y * supersample + sy) * across + x * supersample + sx) * 4;
                    let count = f64::from(cells[cell]);
                    if count <= 0.0 {
                        continue;
                    }
                    for ch in 0..3 {
                        sum[ch] += f64::from(cells[cell + 1 + ch]);
                    }
                    sum[3] += ln_1p(count * normalize) * LOG_SCALE;
                    sum[4] += count;
                }
            }
            if sum[4] > 0.0 {
                for ch in 0..3 {
                    row[x * 4 + ch] = (sum[ch] / (sum[4] * 255.0)).min(1.0) as f32;
                }
                row[x * 4 + 3] = (sum[3] * share) as f32;
            }
        }
    });
    out
}

pub(crate) fn develop(cells: &[f32], across: usize, tone: &Tone, supersample: usize, normalize: f64) -> Vec<f32> {
    let width = across / supersample;
    let height = cells.len() / 4 / across / supersample;
    let inverse_gamma = 1.0 / tone.gamma;
    let share = 1.0 / (supersample * supersample) as f64;

    let mut out = vec![0f32; width * height * 4];
    for_each_row(&mut out, width * 4, |y, row| {
        for x in 0..width {
            let mut sum = [0f64; 4];
            for sy in 0..supersample {
                for sx in 0..supersample {
                    let cell = ((y * supersample + sy) * across + x * supersample + sx) * 4;
                    let count = f64::from(cells[cell]);
                    if count <= 0.0 {
                        continue;
                    }
                    let level = tone.brightness * ln_1p(count * normalize) * LOG_SCALE;
                    let capped = level.min(1.0);
                    let alpha = pow(capped, inverse_gamma);
                    let burn = ((level - 1.0).max(0.0) * tone.highlight).min(1.0);
                    for ch in 0..3 {
                        let average = (f64::from(cells[cell + 1 + ch]) / (count * 255.0)).min(1.0);
                        let vivid = average * alpha;
                        let flat = pow(average * capped, inverse_gamma);
                        let value = flat + (vivid - flat) * tone.vibrancy;
                        sum[ch] += value + (1.0 - value) * burn;
                    }
                    sum[3] += alpha;
                }
            }
            for ch in 0..4 {
                row[x * 4 + ch] = (sum[ch] * share) as f32;
            }
        }
    });

    let sigma = tone.glow_radius * width.min(height) as f64;
    if tone.glow_strength > 0.0 && sigma >= 0.5 {
        glow(&mut out, width, height, sigma, tone.glow_strength as f32);
    }
    out
}

fn glow(image: &mut [f32], width: usize, height: usize, sigma: f64, strength: f32) {
    let reach = (sigma * 3.0).ceil() as usize;
    let mut kernel: Vec<f32> = (0..=reach)
        .map(|i| exp(-((i * i) as f64) / (2.0 * sigma * sigma)) as f32)
        .collect();
    let total: f32 = kernel[0] + 2.0 * kernel[1..].iter().sum::<f32>();
    kernel.iter_mut().for_each(|k| *k /= total);

    let source = image.to_vec();
    let mut across = vec![0f32; image.len()];
    for_each_row(&mut across, width * 4, |y, row| {
        let line = &source[y * width * 4..(y + 1) * width * 4];
        for x in 0..width {
            let (from, to) = (x.saturating_sub(reach), (x + reach).min(width - 1));
            let mut sum = [0f32; 4];
            for sx in from..=to {
                let k = kernel[sx.abs_diff(x)];
                for ch in 0..4 {
                    sum[ch] += line[sx * 4 + ch] * k;
                }
            }
            row[x * 4..x * 4 + 4].copy_from_slice(&sum);
        }
    });
    for_each_row(image, width * 4, |y, row| {
        let (from, to) = (y.saturating_sub(reach), (y + reach).min(height - 1));
        for x in 0..width {
            let mut sum = [0f32; 4];
            for sy in from..=to {
                let k = kernel[sy.abs_diff(y)];
                for ch in 0..4 {
                    sum[ch] += across[(sy * width + x) * 4 + ch] * k;
                }
            }
            for ch in 0..4 {
                row[x * 4 + ch] += sum[ch] * strength;
            }
            row[x * 4 + 3] = row[x * 4 + 3].min(1.0);
        }
    });
}
