use std::collections::HashMap;

use super::{Taste, judged, rounded, with};
use crate::fractal::{Draw, Family, Orbit, Param, Preset, Spec, Walk, random_palette, stream};
use crate::math::{cos, ln, pow, sin};
use crate::palette::Lut;
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "buddhabrot";

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Orbit,
    params: &[
        Param::whole("iterations", 1000.0, 20.0, 100_000.0).log(),
        Param::whole("shortest", 10.0, 1.0, 10_000.0).log(),
        Param::whole("anti", 0.0, 0.0, 1.0).nearest(),
        Param::real("turn", 0.0, -180.0, 180.0),
        Param::real("tilt", 0.0, -180.0, 180.0),
        Param::whole("colored_by", 0.0, 0.0, 2.0),
    ],
    tone: Tone {
        brightness: 2.2,
        gamma: 1.7,
        vibrancy: 1.0,
        highlight: 0.2,
        glow_radius: 0.004,
        glow_strength: 0.3,
        smoothing: 1.0,
    },
    draw: Draw::Density(|params, _| Box::new(Buddhabrot::new(params))),
    random,
    mutate,
    presets: &[
        Preset {
            name: "classic",
            palette: "ice",
            center: [0.0, 0.3],
            zoom: 1.4,
            params: &[],
        },
        Preset {
            name: "nebula",
            palette: "ice",
            center: [0.0, 0.3],
            zoom: 1.25,
            params: &[("iterations", 5000.0), ("shortest", 5.0), ("colored_by", 2.0)],
        },
        Preset {
            name: "filigree",
            palette: "gold",
            center: [0.0, 0.36],
            zoom: 1.85,
            params: &[("iterations", 20_000.0), ("shortest", 2000.0)],
        },
        Preset {
            name: "anti",
            palette: "orchid",
            center: [0.0, 0.39],
            zoom: 2.15,
            params: &[("iterations", 60.0), ("shortest", 5.0), ("anti", 1.0)],
        },
        Preset {
            name: "halfway",
            palette: "ice",
            center: [0.0, 0.56],
            zoom: 1.39,
            params: &[("iterations", 2000.0), ("shortest", 100.0), ("turn", 45.0), ("tilt", 45.0), ("colored_by", 2.0)],
        },
        Preset {
            name: "origins",
            palette: "orchid",
            center: [0.0, 0.36],
            zoom: 1.85,
            params: &[("iterations", 5000.0), ("shortest", 500.0), ("colored_by", 1.0)],
        },
    ],
};

const TASTE: Taste = Taste {
    trimmed_thousandths: 20,
    covered_hundredths: 22,
    cells_holding_half: 20,
    contrast_hundredths: 50,
};

// The upper half of the disc of radius 2 is sampled from two squares of this side.
const SIDE: f64 = 2.0;
const DEPTH: u32 = 8;
const UNITS: u32 = 1 << DEPTH;
const UNIT: f64 = SIDE / UNITS as f64;
const CORNER_LIMIT: u32 = 2048;
// The steps a point may escape later than a corner of its cell: log2(3) from Harnack's inequality, and the rest
// for the count taken at radius 2.
const MARGIN: u32 = 8;
// A cell's farthest point is 0.71 sides from a corner, and has to be within half the corner's clear distance,
// itself at least half the estimate.
const CLEAR: f64 = 2.83;
const FAILURES: u32 = 4_000_000;
// The real parts of the starting points that the second coloring spreads over the palette.
const START: (f64, f64) = (-1.8, 0.45);

#[derive(Clone, Copy)]
struct Corner {
    far: bool,
    part: u8,
    escaped: bool,
    steps: u32,
    reach: f64,
}

#[derive(Clone, Copy)]
struct Cell {
    x: u32,
    y: u32,
    size: u32,
}

struct Buddhabrot {
    limit: u32,
    shortest: u32,
    anti: bool,
    color: u8,
    // The lower two of the three limits of the nebula coloring; the third is `limit`.
    limits: (u32, u32),
    turn: (f64, f64),
    tilt: (f64, f64),
    cells: Vec<Cell>,
    ends: Vec<u64>,
    stride: u64,
}

// 1 inside the main cardioid, 2 inside the disc of period 2, 0 elsewhere.
#[inline]
fn part(re: f64, im: f64) -> u8 {
    let (shifted, im2) = (re - 0.25, im * im);
    let q = shifted * shifted + im2;
    if q * (q + shifted) < 0.25 * im2 {
        1
    } else if (re + 1.0) * (re + 1.0) + im2 < 0.0625 {
        2
    } else {
        0
    }
}

fn corner(re: f64, im: f64, limit: u32) -> Corner {
    let part = part(re, im);
    let mut out = Corner {
        far: re * re + im * im > 4.0,
        part,
        escaped: false,
        steps: limit,
        reach: 0.0,
    };
    if part != 0 {
        return out;
    }
    let (mut x, mut y, mut dx, mut dy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for step in 0..limit + 16 {
        (dx, dy) = (2.0 * (x * dx - y * dy) + 1.0, 2.0 * (x * dy + y * dx));
        (x, y) = (x * x - y * y + re, 2.0 * x * y + im);
        let r2 = x * x + y * y;
        if r2 > 4.0 && !out.escaped {
            if step >= limit {
                break;
            }
            out.escaped = true;
            out.steps = step;
        }
        if r2 > 1e6 {
            out.reach = (r2 / (dx * dx + dy * dy)).sqrt() * 0.5 * ln(r2);
            break;
        }
    }
    out
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

impl Buddhabrot {
    fn new(params: &[f64]) -> Self {
        let limit = params[0] as u32;
        let (turn, tilt) = (params[3].to_radians(), params[4].to_radians());
        let shortest = (params[1] as u32).min(limit / 2).max(1);
        let ratio = f64::from(limit) / f64::from(shortest);
        let third = |thirds: f64| (f64::from(shortest) * pow(ratio, thirds / 3.0)).round() as u32;
        let mut out = Self {
            limit,
            shortest,
            limits: (third(1.0), third(2.0)),
            anti: params[2] >= 0.5,
            color: params[5] as u8,
            turn: (cos(turn), sin(turn)),
            tilt: (cos(tilt), sin(tilt)),
            cells: Vec::new(),
            ends: Vec::new(),
            stride: 1,
        };
        let mut corners = HashMap::new();
        for root in 0..2 {
            let cell = Cell {
                x: root * UNITS,
                y: 0,
                size: UNITS,
            };
            out.divide(cell, &mut corners);
        }
        let mut total = 0;
        out.ends = out
            .cells
            .iter()
            .map(|cell| {
                total += u64::from(cell.size) * u64::from(cell.size);
                total
            })
            .collect();
        out.stride = (total as f64 * 0.618_033_988_749_895) as u64 | 1;
        while gcd(out.stride, total) != 1 {
            out.stride += 2;
        }
        out
    }

    // Keeps the cells that may hold a starting point of an orbit to draw, and no cell that surely holds none.
    fn divide(&mut self, cell: Cell, corners: &mut HashMap<(u32, u32), Corner>) {
        let limit = self.limit.min(CORNER_LIMIT);
        let mut at = |x: u32, y: u32| {
            *corners
                .entry((x, y))
                .or_insert_with(|| corner(-2.0 + f64::from(x) * UNIT, f64::from(y) * UNIT, limit))
        };
        let four = [
            at(cell.x, cell.y),
            at(cell.x + cell.size, cell.y),
            at(cell.x, cell.y + cell.size),
            at(cell.x + cell.size, cell.y + cell.size),
        ];
        let clear = four
            .iter()
            .all(|corner| corner.reach >= CLEAR * f64::from(cell.size) * UNIT);
        let far = four.iter().filter(|corner| corner.far).count();
        let (empty, mixed) = if far == 4 {
            (true, false)
        } else if self.anti {
            let gone = four
                .iter()
                .filter(|corner| corner.escaped && corner.steps + MARGIN < self.limit)
                .count();
            (gone == 4 && clear, gone > 0)
        } else {
            let within = four[0].part != 0 && four.iter().all(|corner| corner.part == four[0].part);
            let fast = four
                .iter()
                .filter(|corner| corner.escaped && corner.steps + MARGIN < self.shortest)
                .count();
            (
                within || (fast == 4 && clear),
                far > 0 || fast > 0 || four.iter().any(|corner| corner.part != 0),
            )
        };
        if empty && cell.size * 8 <= UNITS {
            return;
        }
        if cell.size == 1 || (!mixed && cell.size * 8 <= UNITS) {
            self.cells.push(cell);
            return;
        }
        let half = cell.size / 2;
        for (x, y) in [(0, 0), (half, 0), (0, half), (half, half)] {
            let quarter = Cell {
                x: cell.x + x,
                y: cell.y + y,
                size: half,
            };
            self.divide(quarter, corners);
        }
    }

    // The steps an orbit to draw stays within radius 2, or nothing for an orbit that is not drawn.
    fn length(&self, re: f64, im: f64) -> Option<u32> {
        let inside = part(re, im) != 0;
        if inside {
            return self.anti.then_some(self.limit);
        }
        let (mut x, mut y) = (0.0f64, 0.0f64);
        let (mut old_x, mut old_y, mut lap, mut period) = (f64::NAN, f64::NAN, 0u32, 8u32);
        for step in 0..self.limit {
            (x, y) = (x * x - y * y + re, 2.0 * x * y + im);
            if x * x + y * y > 4.0 {
                return (!self.anti && step >= self.shortest).then_some(step);
            }
            // An orbit that comes back to where it was never leaves.
            if x == old_x && y == old_y {
                break;
            }
            lap += 1;
            if lap == period {
                (old_x, old_y, lap, period) = (x, y, 0, period.saturating_mul(2));
            }
        }
        self.anti.then_some(self.limit)
    }

    #[inline]
    fn class(&self, count: u32) -> f64 {
        if count <= self.limits.0 {
            1.0
        } else if count <= self.limits.1 {
            0.5
        } else {
            0.0
        }
    }
}

struct Trail<'a> {
    map: &'a Buddhabrot,
    cursor: u64,
    c: (f64, f64),
    z: (f64, f64),
    length: u32,
    // The steps plotted: an orbit that leaves is followed one step past radius 2, so that no rim shows there.
    last: u32,
    step: u32,
    mirrored: bool,
    shade: f64,
}

impl Orbit for Buddhabrot {
    fn walk(&self, rng: &mut Rng) -> Box<dyn Walk + '_> {
        Box::new(Trail {
            map: self,
            cursor: rng.below(self.ends.last().copied().unwrap_or(1)),
            c: (0.0, 0.0),
            z: (0.0, 0.0),
            length: 0,
            last: 0,
            step: 0,
            mirrored: false,
            shade: 0.0,
        })
    }

    fn colors(&self) -> Option<Lut> {
        if self.color != 2 {
            return None;
        }
        let mut lut = [[255, 0, 0]; 256];
        lut[64..192].fill([170, 255, 0]);
        lut[192..].fill([85, 128, 255]);
        Some(lut)
    }
}

impl Trail<'_> {
    fn next_orbit(&mut self, rng: &mut Rng) -> bool {
        let map = self.map;
        let Some(total) = map.ends.last().copied() else {
            return false;
        };
        for _ in 0..FAILURES {
            self.cursor = (self.cursor + map.stride) % total;
            let cell = map.cells[map.ends.partition_point(|end| *end <= self.cursor)];
            let re = -2.0 + (f64::from(cell.x) + rng.next_f64() * f64::from(cell.size)) * UNIT;
            let im = (f64::from(cell.y) + rng.next_f64() * f64::from(cell.size)) * UNIT;
            if let Some(length) = map.length(re, im)
                && length > 0
            {
                self.c = (re, im);
                self.z = (0.0, 0.0);
                self.length = length;
                self.last = if map.anti { length } else { length + 1 };
                self.step = 0;
                self.mirrored = false;
                self.shade = if map.color == 2 {
                    map.class(length)
                } else if map.color == 1 {
                    ((re - START.0) / (START.1 - START.0)).clamp(0.0, 1.0)
                } else {
                    let lowest = f64::from(map.shortest);
                    (ln(f64::from(length) / lowest) / ln(f64::from(map.limit) / lowest)).clamp(0.0, 1.0)
                };
                return true;
            }
        }
        false
    }
}

impl Walk for Trail<'_> {
    fn run(&mut self, rng: &mut Rng, count: u64, plot: &mut dyn FnMut(f64, f64, f64)) {
        let map = self.map;
        let own = !map.anti || map.color == 1;
        let mut left = count;
        while left > 0 {
            if !self.mirrored {
                if self.step == self.last && !self.next_orbit(rng) {
                    for _ in 0..left {
                        plot(f64::NAN, f64::NAN, 0.0);
                    }
                    return;
                }
                let (x, y) = self.z;
                self.z = (x * x - y * y + self.c.0, 2.0 * x * y + self.c.1);
                self.step += 1;
            }
            let (re, im) = self.z;
            let across = im * map.turn.0 + self.c.1 * map.turn.1;
            let up = -(re * map.tilt.0 + self.c.0 * map.tilt.1);
            let place = if own {
                self.shade
            } else if map.color == 2 {
                map.class(self.step)
            } else {
                ln(f64::from(self.step)) / ln(f64::from(map.limit))
            };
            plot(if self.mirrored { -across } else { across }, up, place);
            self.mirrored = !self.mirrored;
            left -= 1;
        }
    }
}

fn frame(values: &[f64]) -> Option<(Camera, bool)> {
    let map = Buddhabrot::new(values);
    // An odd stride takes the points and their mirrored twins in turn.
    let (mut camera, good) = judged(&map, u64::from(map.limit / 20) | 1, &TASTE)?;
    camera.center[0] = 0.0;
    Some((camera, good))
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..8 {
        let mut rng = stream(NAME, seed, attempt);
        let anti = if rng.next_f64() < 0.15 { 1.0 } else { 0.0 };
        let iterations = if anti == 1.0 {
            (20.0 * pow(10.0, rng.next_f64())).round()
        } else {
            (300.0 * pow(30.0, rng.next_f64())).round()
        };
        let shortest = if anti == 1.0 || rng.next_f64() < 0.3 {
            5.0
        } else {
            (iterations / rng.range(4.0, 40.0)).round().max(5.0)
        };
        let (mut turn, mut tilt) = (0.0, 0.0);
        if rng.next_f64() < 0.35 {
            turn = rng.range(-90.0, 90.0);
            tilt = rng.range(-90.0, 90.0);
        }
        let color = rng.below(3) as f64;
        let values = rounded(&SPEC, vec![iterations, shortest, anti, turn, tilt, color]);
        let colors = random_palette(&mut rng);
        match frame(&values) {
            Some((camera, true)) => return with(&SPEC, colors, camera, &values),
            Some((camera, false)) => {
                fallback.get_or_insert_with(|| with(&SPEC, colors, camera, &values));
            }
            None => {}
        }
    }
    fallback.unwrap_or_else(|| SPEC.fractal("ice", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let before = SPEC.values(fractal);
    let scale = |rng: &mut Rng| pow(2.0, rng.range(-1.0, 1.0) * strength);
    let values = rounded(
        &SPEC,
        vec![
            before[0] * scale(rng),
            before[1] * scale(rng),
            before[2],
            before[3] + rng.range(-25.0, 25.0) * strength,
            before[4] + rng.range(-25.0, 25.0) * strength,
            before[5],
        ],
    );
    let mut out = fractal.clone();
    if let Some((camera, _)) = frame(&values) {
        out.camera = Camera {
            rotation: fractal.camera.rotation,
            flip_y: fractal.camera.flip_y,
            ..camera
        };
    }
    out.params = SPEC.params.iter().map(|param| param.name.to_owned()).zip(values).collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    #[test]
    fn the_cells_left_out_hold_no_orbit_to_draw() {
        for params in [
            [400.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [400.0, 100.0, 0.0, 0.0, 0.0, 0.0],
            [3000.0, 1500.0, 0.0, 0.0, 0.0, 0.0],
            [300.0, 5.0, 1.0, 0.0, 0.0, 0.0],
        ] {
            let map = Buddhabrot::new(&params);
            let wide = 2 * UNITS as usize;
            let mut kept = vec![false; wide * UNITS as usize];
            for cell in &map.cells {
                for y in cell.y..cell.y + cell.size {
                    for x in cell.x..cell.x + cell.size {
                        kept[y as usize * wide + x as usize] = true;
                    }
                }
            }
            assert!(kept.iter().any(|kept| !kept), "{params:?}");
            let mut rng = Rng::derive(11, 0, 0);
            for _ in 0..200_000 {
                let (re, im) = (rng.range(-2.0, 2.0), rng.range(0.0, 2.0));
                if map.length(re, im).is_some_and(|length| length > 0) {
                    let cell = (im / UNIT) as usize * wide + ((re + 2.0) / UNIT) as usize;
                    assert!(kept[cell], "{params:?}: {re} {im}");
                }
            }
        }
    }


    #[test]
    fn the_picture_is_its_own_mirror_image() {
        let map = Buddhabrot::new(&[200.0, 10.0, 0.0, 30.0, 60.0, 1.0]);
        let mut rng = Rng::derive(3, 0, 0);
        let mut points = Vec::new();
        map.walk(&mut rng).run(&mut rng, 1000, &mut |x, y, place| points.push((x, y, place)));
        for pair in points.chunks_exact(2) {
            assert_eq!((pair[0].0, pair[0].1, pair[0].2), (-pair[1].0, pair[1].1, pair[1].2));
        }
    }

    #[test]
    fn the_nebula_brings_its_own_three_colors() {
        assert!(Buddhabrot::new(&[5000.0, 5.0, 0.0, 0.0, 0.0, 0.0]).colors().is_none());
        let map = Buddhabrot::new(&[5000.0, 5.0, 0.0, 0.0, 0.0, 2.0]);
        assert_eq!(map.limits, (50, 500));
        let colors = map.colors().unwrap();
        assert_eq!((colors[0], colors[128], colors[255]), ([255, 0, 0], [170, 255, 0], [85, 128, 255]));
        assert_eq!((map.class(50), map.class(51), map.class(500), map.class(501)), (1.0, 0.5, 0.5, 0.0));
    }

    #[test]
    fn it_renders_to_its_golden_hashes() {
        for (name, expected) in [
            ("buddhabrot-classic", 0xe391_90ce_a9a1_4354u64),
            ("buddhabrot-nebula", 0x54bb_743c_f970_61e5),
            ("buddhabrot-anti", 0x2209_36d2_1d97_05b5),
            ("buddhabrot-halfway", 0xccc1_cee6_f6a7_0d02),
        ] {
            let hash = golden(&preset(name).unwrap());
            assert_eq!(hash, expected, "{name}: got {hash:#018x}");
        }
    }
}
