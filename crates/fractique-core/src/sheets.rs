use crate::math::{TAU, cos, exp, sin};
use crate::scene::{Shape, Sheet, Sheets};

const STEP: f64 = 1e-3;

enum Form {
    Box {
        half: [f64; 2],
        corners: [f64; 4],
    },
    Petal {
        radius: f64,
        offset: f64,
        tip: f64,
    },
    Ring {
        radius: f64,
        half: f64,
        open: Option<(f64, f64)>,
    },
    Wave {
        swells: Vec<(f64, f64, f64)>,
        half: f64,
    },
}

struct Solid {
    form: Form,
    center: (f64, f64),
    turn: (f64, f64),
    reach: f64,
    extent: (f64, f64),
}

impl Solid {
    fn new(shape: &Shape) -> Self {
        let turn = |degrees: f64| {
            let angle = degrees.to_radians();
            (cos(angle), sin(angle))
        };
        match shape {
            Shape::Box {
                center,
                half,
                corners,
                rotation,
            } => Self {
                form: Form::Box {
                    half: *half,
                    corners: *corners,
                },
                center: (center[0], center[1]),
                turn: turn(*rotation),
                reach: (half[0] * half[0] + half[1] * half[1]).sqrt(),
                extent: (half[0], half[1]),
            },
            Shape::Petal {
                base,
                length,
                width,
                rotation,
            } => {
                let (a, b) = (width / 2.0, length / 2.0);
                let radius = (a + b * b / a) / 2.0;
                let (c, s) = turn(*rotation);
                Self {
                    form: Form::Petal {
                        radius,
                        offset: radius - a,
                        tip: b,
                    },
                    center: (base[0] + s * b, base[1] - c * b),
                    turn: (c, s),
                    reach: b,
                    extent: (a, b),
                }
            }
            Shape::Ring {
                center,
                radius,
                thickness,
                span,
                rotation,
            } => {
                let outer = radius + thickness / 2.0;
                Self {
                    form: Form::Ring {
                        radius: *radius,
                        half: thickness / 2.0,
                        open: (*span < 360.0).then(|| {
                            let (c, s) = turn(span / 2.0);
                            (s, c)
                        }),
                    },
                    center: (center[0], center[1]),
                    turn: turn(*rotation),
                    reach: outer,
                    extent: (outer, outer),
                }
            }
            Shape::Wave {
                level,
                swells,
                thickness,
                rotation,
            } => Self {
                form: Form::Wave {
                    swells: swells.iter().map(|s| (s.height, TAU / s.length, s.shift)).collect(),
                    half: thickness / 2.0,
                },
                center: (0.0, *level),
                turn: turn(*rotation),
                reach: f64::INFINITY,
                extent: (f64::INFINITY, f64::INFINITY),
            },
        }
    }

    fn local(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.center.0, y - self.center.1);
        let (c, s) = self.turn;
        (dx * c + dy * s, dy * c - dx * s)
    }

    fn distance(&self, x: f64, y: f64, far: f64) -> f64 {
        if self.reach.is_finite() {
            let (dx, dy) = (x - self.center.0, y - self.center.1);
            let away = (dx * dx + dy * dy).sqrt() - self.reach;
            if away > far {
                return away;
            }
        }
        let (px, py) = self.local(x, y);
        match &self.form {
            Form::Box { half, corners } => {
                let [hx, hy] = *half;
                let (qx, qy) = (px.abs() - hx, py.abs() - hy);
                let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                for (radius, (sx, sy)) in corners.iter().zip(signs) {
                    if *radius <= 0.0 {
                        continue;
                    }
                    let (vx, vy) = ((px - sx * (hx - radius)) * sx, (py - sy * (hy - radius)) * sy);
                    if vx > 0.0 && vy > 0.0 {
                        let round = (vx * vx + vy * vy).sqrt() - radius;
                        return if qx <= 0.0 && qy <= 0.0 {
                            round.max(qx.max(qy))
                        } else {
                            round
                        };
                    }
                }
                let (ox, oy) = (qx.max(0.0), qy.max(0.0));
                (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0)
            }
            Form::Petal { radius, offset, tip } => {
                let (ax, ay) = (px.abs(), py.abs());
                if (ay - tip) * offset > ax * tip {
                    (ax * ax + (ay - tip) * (ay - tip)).sqrt()
                } else {
                    ((ax + offset) * (ax + offset) + ay * ay).sqrt() - radius
                }
            }
            Form::Ring { radius, half, open } => {
                let ax = px.abs();
                match open {
                    Some((s, c)) if c * ax > s * -py => {
                        let (ex, ey) = (ax - s * radius, -py - c * radius);
                        (ex * ex + ey * ey).sqrt() - half
                    }
                    _ => ((ax * ax + py * py).sqrt() - radius).abs() - half,
                }
            }
            Form::Wave { swells, half } => {
                let (mut height, mut slope) = (0.0, 0.0);
                for (amplitude, rate, shift) in swells {
                    let phase = px * rate + shift;
                    height += amplitude * sin(phase);
                    slope += amplitude * rate * cos(phase);
                }
                let gap = (height - py) / (1.0 + slope * slope).sqrt();
                if *half > 0.0 { gap.abs() - half } else { gap }
            }
        }
    }

    fn across(&self, x: f64, y: f64, light: (f64, f64)) -> f64 {
        if !self.reach.is_finite() {
            return 0.0;
        }
        let (px, py) = self.local(x, y);
        let (c, s) = self.turn;
        let (lx, ly) = (light.0 * c + light.1 * s, light.1 * c - light.0 * s);
        (px / self.extent.0).clamp(-1.0, 1.0) * lx + (py / self.extent.1).clamp(-1.0, 1.0) * ly
    }
}

struct Ply<'a> {
    sheet: &'a Sheet,
    solids: Vec<Solid>,
}

impl Ply<'_> {
    fn nearest(&self, x: f64, y: f64, far: f64) -> (f64, &Solid) {
        let mut best = (f64::INFINITY, &self.solids[0]);
        for solid in &self.solids {
            let d = solid.distance(x, y, far);
            if d < best.0 {
                best = (d, solid);
            }
        }
        best
    }
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

struct Frame<'a> {
    sheets: &'a Sheets,
    plies: Vec<Ply<'a>>,
    width: usize,
    scale: f64,
    half: (f64, f64),
    light: (f64, f64),
}

impl Frame<'_> {
    fn row(&self, y: usize, out: &mut [f32]) {
        let sheets = self.sheets;
        let (lx, ly) = self.light;
        let py = (y as f64 + 0.5) * self.scale - self.half.1;
        let reach = self.half.0.max(self.half.1);
        let far = sheets.shadow_blur + sheets.shadow_offset + 2.0 * self.scale;
        for (x, pixel) in out.chunks_exact_mut(3).enumerate() {
            let px = (x as f64 + 0.5) * self.scale - self.half.0;
            let slope = (px * lx + py * ly) / reach;
            let mut color = [f64::from(pixel[0]), f64::from(pixel[1]), f64::from(pixel[2])];
            for ply in &self.plies {
                let sheet = ply.sheet;
                let (cast, _) = ply.nearest(px + lx * sheets.shadow_offset, py + ly * sheets.shadow_offset, far);
                let shade = 1.0 - sheets.shadow * sheet.opacity * smooth(0.5 - cast / (2.0 * sheets.shadow_blur));
                color = color.map(|c| c * shade);

                let (d, solid) = ply.nearest(px, py, far);
                let cover = (0.5 - d / self.scale).clamp(0.0, 1.0) * sheet.opacity;
                if cover <= 0.0 {
                    continue;
                }
                let (gx, gy) = (
                    solid.distance(px + STEP, py, far) - d,
                    solid.distance(px, py + STEP, far) - d,
                );
                let length = (gx * gx + gy * gy).sqrt();
                let facing = if length > 0.0 {
                    (gx * lx + gy * ly) / length
                } else {
                    0.0
                };
                let lit = (1.0 + sheets.bevel * solid.across(px, py, self.light)) * (1.0 + sheets.gradient * slope);
                let rim = sheets.rim * exp(d.min(0.0) / sheets.rim_width) * (0.4 + 0.6 * facing.max(0.0));
                let blend = (0.5 - 0.5 * slope).clamp(0.0, 1.0);
                for (ch, value) in color.iter_mut().enumerate() {
                    let base = match sheet.color_to {
                        Some(to) => sheet.color[ch] + (to[ch] - sheet.color[ch]) * blend,
                        None => sheet.color[ch],
                    };
                    *value += (base * lit + rim - *value) * cover;
                }
            }
            for (ch, value) in color.into_iter().enumerate() {
                pixel[ch] = value as f32;
            }
        }
    }
}

pub(crate) fn draw(sheets: &Sheets, canvas: &mut [f32], width: usize, height: usize) {
    let scale = 2.0 / width.min(height) as f64;
    let angle = sheets.light.to_radians();
    let frame = Frame {
        sheets,
        plies: sheets
            .stack
            .iter()
            .map(|sheet| Ply {
                sheet,
                solids: sheet.shapes.iter().map(Solid::new).collect(),
            })
            .collect(),
        width,
        scale,
        half: (width as f64 * scale / 2.0, height as f64 * scale / 2.0),
        light: (cos(angle), -sin(angle)),
    };
    let run = |(y, row): (usize, &mut [f32])| frame.row(y, row);
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        canvas.par_chunks_exact_mut(frame.width * 3).enumerate().for_each(run);
    }
    #[cfg(not(feature = "parallel"))]
    canvas.chunks_exact_mut(frame.width * 3).enumerate().for_each(run);
}
