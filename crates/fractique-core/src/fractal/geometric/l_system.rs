use super::{COLOR_DENSITY, COLOR_SHIFT, Lines, Segment, WIDTH, folded, number, recolored};
use crate::fractal::density;
use crate::fractal::{Draw, Family, Param, Preset, Spec, Text, random_palette, stream};
use crate::math::{cos, fmod, sin};
use crate::random::GRID;
use crate::rng::Rng;
use crate::scene::{Camera, Fractal, Tone};

const NAME: &str = "l_system";
const ATTEMPTS: u64 = 40;
const AXIOM: &str = "axiom";
const RULES: &str = "rules";
const LONGEST_AXIOM: usize = 200;
const LONGEST_RULES: usize = 1000;
const MOST_SEGMENTS: f64 = 1_048_576.0;
const MOST_SYMBOLS: f64 = 16_000_000.0;
const DEEPEST_BRANCH: usize = 100_000;
// The longer side of the drawing, in units of the plane: a picture's shorter side shows 4 of them at zoom 1.
const SPAN: f64 = 3.0;

const ITERATIONS: Param = Param::whole("iterations", 12.0, 0.0, 30.0);
const ANGLE: Param = Param::real("angle", 90.0, 0.0, 180.0);
const COLOR_FROM: Param = Param::whole("color_from", 0.0, 0.0, 2.0);

pub(super) static SPEC: Spec = Spec {
    name: NAME,
    family: Family::Geometric,
    params: &[ITERATIONS, ANGLE, WIDTH, COLOR_FROM, COLOR_DENSITY, COLOR_SHIFT],
    tone: Tone {
        brightness: 1.0,
        gamma: 2.6,
        vibrancy: 1.0,
        highlight: 0.15,
        glow_radius: 0.005,
        glow_strength: 0.45,
        smoothing: 0.6,
    },
    draw: Draw::DensityText(
        &[
            Text {
                name: AXIOM,
                default: "FX",
                check: |axiom| written(axiom, LONGEST_AXIOM, "="),
                presets: &[
                    ("koch-snowflake", "F--F--F"),
                    ("sierpinski", "F-G-G"),
                    ("hilbert", "A"),
                    ("gosper", "F"),
                    ("plant", "X"),
                    ("bush", "F"),
                ],
            },
            Text {
                name: RULES,
                default: "X=X+YF+,Y=-FX-Y",
                check: |rules| rules_of(rules).map(|_| ()),
                presets: &[
                    ("koch-snowflake", "F=F+F--F+F"),
                    ("sierpinski", "F=F-G+F+G-F,G=GG"),
                    ("hilbert", "A=-BF+AFA+FB-,B=+AF-BFB-FA+"),
                    ("gosper", "F=F-G--G+F++FF+G-,G=+F-GG--G-F++F+G"),
                    ("plant", "X=F+[[X]-X]-F[-FX]+X,F=FF"),
                    ("bush", "F=FF-[-F+F+F]+[+F-F-F]"),
                ],
            },
        ],
        |fractal| Box::new(drawing(fractal)),
    ),
    random,
    mutate,
    presets: &[
        Preset {
            name: "dragon",
            palette: "sunset",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 13.0), ("angle", 90.0)],
        },
        Preset {
            name: "koch-snowflake",
            palette: "ice",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 5.0), ("angle", 60.0)],
        },
        Preset {
            name: "sierpinski",
            palette: "gold",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 7.0), ("angle", 120.0)],
        },
        Preset {
            name: "hilbert",
            palette: "aurora",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 6.0), ("angle", 90.0)],
        },
        Preset {
            name: "gosper",
            palette: "orchid",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 4.0), ("angle", 60.0)],
        },
        Preset {
            name: "plant",
            palette: "emerald",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 6.0), ("angle", 25.0), ("color_from", 1.0), ("color_shift", 0.3)],
        },
        Preset {
            name: "bush",
            palette: "emerald",
            center: [0.0, 0.0],
            zoom: 1.15,
            params: &[("iterations", 4.0), ("angle", 22.5), ("color_from", 1.0)],
        },
    ],
};

fn written(text: &str, longest: usize, besides: &str) -> Result<(), String> {
    if text.is_empty() || text.len() > longest {
        return Err(format!("it holds 1 to {longest} characters"));
    }
    match text
        .chars()
        .find(|symbol| !symbol.is_ascii_graphic() || ",;".contains(*symbol) || besides.contains(*symbol))
    {
        Some(symbol) => Err(format!("`{symbol}` is not a symbol")),
        None => Ok(()),
    }
}

// The rules by the symbol they rewrite: `F=F+F--F+F`, several apart by commas, semicolons or spaces.
fn rules_of(rules: &str) -> Result<[Option<&[u8]>; 128], String> {
    if rules.len() > LONGEST_RULES {
        return Err(format!("it holds at most {LONGEST_RULES} characters"));
    }
    let mut table = [None; 128];
    for rule in rules
        .split(|symbol: char| ",;".contains(symbol) || symbol.is_whitespace())
        .filter(|rule| !rule.is_empty())
    {
        let Some((symbol, becomes)) = rule.split_once('=') else {
            return Err(format!("`{rule}` is not a rule like F=F+F"));
        };
        written(symbol, 1, "=").map_err(|_| format!("`{rule}` rewrites one symbol"))?;
        if !becomes.is_empty() {
            written(becomes, LONGEST_RULES, "=").map_err(|e| format!("`{rule}`: {e}"))?;
        }
        let slot = &mut table[usize::from(symbol.as_bytes()[0])];
        if slot.is_some() {
            return Err(format!("`{symbol}` has two rules"));
        }
        *slot = Some(becomes.as_bytes());
    }
    Ok(table)
}

// How many times the rules are applied: as many as asked, while the drawing stays within its limits.
fn depth(axiom: &[u8], rules: &[Option<&[u8]>; 128], asked: u32) -> u32 {
    let mut counts = [0f64; 128];
    for symbol in axiom {
        counts[usize::from(*symbol)] += 1.0;
    }
    for depth in 0..asked {
        let mut next = [0f64; 128];
        for (symbol, count) in counts.iter().enumerate().filter(|(_, count)| **count > 0.0) {
            match rules[symbol] {
                Some(becomes) => becomes.iter().for_each(|symbol| next[usize::from(*symbol)] += count),
                None => next[symbol] += count,
            }
        }
        let drawn = next[usize::from(b'F')] + next[usize::from(b'G')];
        if drawn > MOST_SEGMENTS || next.iter().sum::<f64>() > MOST_SYMBOLS {
            return depth;
        }
        counts = next;
    }
    asked
}

struct Stroke {
    from: (f64, f64),
    to: (f64, f64),
    branch: u32,
    turns: i64,
}

// The turtle: F and G draw a step, f and g move one, + turns left and - right by the angle, | turns back,
// [ remembers the place and the heading and ] returns to them. Every other symbol only stands for its rule.
fn strokes(axiom: &[u8], rules: &[Option<&[u8]>; 128], depth: u32, angle: f64) -> Vec<Stroke> {
    let mut strokes = Vec::new();
    let mut pending: Vec<(&[u8], usize, u32)> = vec![(axiom, 0, depth)];
    let mut kept: Vec<((f64, f64), i64, f64)> = Vec::new();
    let (mut at, mut turns, mut reversed) = ((0.0f64, 0.0f64), 0i64, 0.0f64);
    let heading = |turns: i64, reversed: f64| {
        let degrees = fmod(90.0 + reversed + fmod(turns as f64 * angle, 360.0), 360.0).to_radians();
        (cos(degrees), sin(degrees))
    };
    let mut step = heading(turns, reversed);
    while let Some((symbols, next, left)) = pending.last_mut() {
        let Some(symbol) = symbols.get(*next).copied() else {
            pending.pop();
            continue;
        };
        *next += 1;
        if *left > 0
            && let Some(becomes) = rules[usize::from(symbol)]
        {
            let left = *left - 1;
            pending.push((becomes, 0, left));
            continue;
        }
        match symbol {
            b'F' | b'G' | b'f' | b'g' => {
                let to = (at.0 + step.0, at.1 + step.1);
                if symbol.is_ascii_uppercase() {
                    strokes.push(Stroke {
                        from: at,
                        to,
                        branch: kept.len() as u32,
                        turns,
                    });
                }
                at = to;
            }
            b'+' | b'-' => {
                turns += if symbol == b'+' { 1 } else { -1 };
                step = heading(turns, reversed);
            }
            b'|' => {
                reversed = 180.0 - reversed;
                step = heading(turns, reversed);
            }
            b'[' if kept.len() < DEEPEST_BRANCH => kept.push((at, turns, reversed)),
            b']' => {
                if let Some(state) = kept.pop() {
                    (at, turns, reversed) = state;
                    step = heading(turns, reversed);
                }
            }
            _ => {}
        }
    }
    strokes
}

fn drawing(fractal: &Fractal) -> Lines {
    let values = SPEC.values(fractal);
    let axiom = SPEC.text(fractal, AXIOM).as_bytes();
    let rules = rules_of(SPEC.text(fractal, RULES)).unwrap_or([None; 128]);
    let (angle, coloring, density, shift) = (values[1], values[3] as u32, values[4], values[5]);
    let strokes = strokes(axiom, &rules, depth(axiom, &rules, values[0] as u32), angle);

    let (mut low, mut high) = ((f64::INFINITY, f64::INFINITY), (f64::NEG_INFINITY, f64::NEG_INFINITY));
    for point in strokes.iter().flat_map(|stroke| [stroke.from, stroke.to]) {
        low = (low.0.min(point.0), low.1.min(point.1));
        high = (high.0.max(point.0), high.1.max(point.1));
    }
    let scale = SPAN / (high.0 - low.0).max(high.1 - low.1).max(1e-9);
    let middle = ((low.0 + high.0) * 0.5, (low.1 + high.1) * 0.5);
    let placed = |point: (f64, f64)| ((point.0 - middle.0) * scale, (point.1 - middle.1) * scale);

    let deepest = strokes.iter().map(|stroke| stroke.branch).max().unwrap_or(0).max(1);
    let count = strokes.len().max(1) as f64;
    let color = |share: f64| folded(shift + density * share) as f32;
    let segments = strokes
        .iter()
        .enumerate()
        .map(|(index, stroke)| {
            let colors = match coloring {
                0 => (color(index as f64 / count), color((index + 1) as f64 / count)),
                1 => {
                    let share = color(f64::from(stroke.branch) / f64::from(deepest));
                    (share, share)
                }
                _ => {
                    let share = color(fmod(stroke.turns as f64 * angle, 360.0).abs() / 180.0);
                    (share, share)
                }
            };
            Segment {
                from: placed(stroke.from),
                to: placed(stroke.to),
                colors,
            }
        })
        .collect();
    Lines::new(segments, values[2], &fractal.camera)
}

struct Kind {
    axiom: &'static str,
    rules: &'static str,
    angle: f64,
    branches: bool,
}

const fn curve(axiom: &'static str, rules: &'static str, angle: f64) -> Kind {
    Kind {
        axiom,
        rules,
        angle,
        branches: false,
    }
}

const fn plant(axiom: &'static str, rules: &'static str, angle: f64) -> Kind {
    Kind {
        axiom,
        rules,
        angle,
        branches: true,
    }
}

// What the randomizer starts from: the curves and plants of "The Algorithmic Beauty of Plants" and their kin.
const KINDS: [Kind; 26] = [
    curve("FX", "X=X+YF+,Y=-FX-Y", 90.0),
    curve("F--F--F", "F=F+F--F+F", 60.0),
    curve("F-G-G", "F=F-G+F+G-F,G=GG", 120.0),
    curve("XF", "X=YF+XF+Y,Y=XF-YF-X", 60.0),
    curve("A", "A=-BF+AFA+FB-,B=+AF-BFB-FA+", 90.0),
    curve("F", "F=F-G--G+F++FF+G-,G=+F-GG--G-F++F+G", 60.0),
    curve("F", "F=+F--F+", 45.0),
    curve("F", "F=F+F-F", 120.0),
    curve("X", "X=XFYFX+F+YFXFY-F-XFYFX,Y=YFXFY-F-XFYFX+F+YFXFY", 90.0),
    curve("F-F-F-F", "F=F-F+F+FF-F-F+F", 90.0),
    curve("F-F-F-F", "F=FF-F-F-F-F-F+F", 90.0),
    curve("F-F-F-F", "F=FF-F-F-F-FF", 90.0),
    curve("F-F-F-F", "F=FF-F+F-F-FF", 90.0),
    curve("F-F-F-F", "F=FF-F--F-F", 90.0),
    curve("F-F-F-F", "F=F-FF--F-F", 90.0),
    curve("F-F-F-F", "F=F-F+F-F-F", 90.0),
    curve("F+F+F+F", "F=FF+F++F+F", 90.0),
    curve("F++F++F++F++F", "F=F++F++F|F-F++F", 36.0),
    curve("F+F+F+F", "F=F+f-FF+F+FF+Ff+FF-f+FF-F-FF-Ff-FFF,f=ffffff", 90.0),
    curve("-G", "F=F+G++G-F--FF-G+,G=-F+GG++G+F--F-G", 60.0),
    plant("F", "F=F[+F]F[-F]F", 25.7),
    plant("F", "F=F[+F]F[-F][F]", 20.0),
    plant("F", "F=FF-[-F+F+F]+[+F-F-F]", 22.5),
    plant("X", "X=F[+X]F[-X]+X,F=FF", 20.0),
    plant("X", "X=F[+X][-X]FX,F=FF", 25.7),
    plant("X", "X=F-[[X]+X]+F[+FX]-X,F=FF", 22.5),
];

// A drawing worth a picture: not a line, not a blot, with neighbouring cells that differ. A curve is a blot
// where its usual cell holds more than three lines side by side; a plant may be as dense as it grows.
fn good(lines: &Lines, branches: bool) -> bool {
    if lines.len() < 400 {
        return false;
    }
    let Some(probe) = density::probe(lines) else {
        return false;
    };
    let mut covered: Vec<u32> = probe.cells.iter().copied().filter(|hits| *hits > 0).collect();
    covered.sort_unstable();
    let cell = 4.0 / probe.camera.zoom / GRID as f64;
    let usual = f64::from(covered[covered.len() / 2]) / probe.points as f64 * lines.length() / cell;
    covered.len() * 100 >= GRID * GRID * 9
        && covered.len() * 100 <= GRID * GRID * 70
        && probe.contrast() * 100 >= probe.inside * 25
        && (branches || usual <= 3.2)
}

fn random(seed: u64) -> Fractal {
    let mut fallback = None;
    for attempt in 0..ATTEMPTS {
        let mut rng = stream(NAME, seed, attempt);
        let kind = &KINDS[rng.below(KINDS.len() as u64) as usize];
        let turned = rng.next_f64() < 0.55;
        let angle = if !turned {
            kind.angle
        } else if kind.branches {
            (kind.angle + rng.range(-10.0, 14.0)).round()
        } else {
            (kind.angle * rng.range(0.82, 1.18) * 2.0).round() / 2.0
        };
        let rules = rules_of(kind.rules).unwrap_or([None; 128]);
        let most = depth(kind.axiom.as_bytes(), &rules, 30);
        // The finest drawing whose steps stay apart: a curve of steps a 110th of its span is a blot.
        let finest = if kind.branches { 500.0 } else { 110.0 };
        let mut iterations = 1;
        for deeper in 2..=most {
            let strokes = strokes(kind.axiom.as_bytes(), &rules, deeper, angle);
            let (mut low, mut high) = ((0.0f64, 0.0f64), (0.0f64, 0.0f64));
            for stroke in &strokes {
                low = (low.0.min(stroke.to.0), low.1.min(stroke.to.1));
                high = (high.0.max(stroke.to.0), high.1.max(stroke.to.1));
            }
            if strokes.len() > 200_000 || (high.0 - low.0).max(high.1 - low.1) > finest {
                break;
            }
            iterations = deeper;
        }
        let params = [
            ("iterations", f64::from(iterations)),
            ("angle", angle),
            ("color_from", if kind.branches && rng.next_f64() < 0.7 { 1.0 } else { rng.below(3) as f64 }),
            ("color_density", (rng.range(0.7, 2.0) * 100.0).round() / 100.0),
            ("color_shift", (rng.range(0.0, 2.0) * 100.0).round() / 100.0),
        ];
        let camera = Camera {
            zoom: 1.15,
            ..Camera::default()
        };
        let mut fractal = SPEC.fractal(random_palette(&mut rng), camera, &params);
        fractal.text.insert(AXIOM.to_owned(), kind.axiom.to_owned());
        fractal.text.insert(RULES.to_owned(), kind.rules.to_owned());
        if good(&drawing(&fractal), kind.branches) {
            return fractal;
        }
        fallback.get_or_insert(fractal);
    }
    fallback.unwrap_or_else(|| SPEC.fractal("sunset", Camera::default(), &[]))
}

fn mutate(fractal: &Fractal, rng: &mut Rng, strength: f64) -> Fractal {
    let mut out = fractal.clone();
    for _ in 0..8 {
        let angle = (number(fractal, &ANGLE) + rng.range(-9.0, 9.0) * strength).clamp(ANGLE.min, ANGLE.max);
        out.params.insert(ANGLE.name.to_owned(), (angle * 10.0).round() / 10.0);
        if good(&drawing(&out), SPEC.text(fractal, RULES).contains('[')) {
            break;
        }
    }
    recolored(&mut out, rng, strength);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fractal::{golden, preset};

    fn drawn(axiom: &str, rules: &str, depth: u32, angle: f64) -> Vec<Stroke> {
        strokes(axiom.as_bytes(), &rules_of(rules).unwrap(), depth, angle)
    }

    #[test]
    fn the_curves_have_the_lengths_and_the_ends_of_the_books() {
        for order in 0..6 {
            let koch = drawn("F", "F=F+F--F+F", order, 60.0);
            assert_eq!(koch.len(), 4usize.pow(order));
            let end = koch.last().unwrap().to;
            assert!((end.0.abs() + (end.1 - 3f64.powi(order as i32)).abs()) < 1e-9, "{end:?}");
        }
        assert_eq!(drawn("FX", "X=X+YF+,Y=-FX-Y", 10, 90.0).len(), 1024);
        assert_eq!(drawn("A", "A=-BF+AFA+FB-,B=+AF-BFB-FA+", 5, 90.0).len(), 4usize.pow(5) - 1);
        let hilbert = drawn("A", "A=-BF+AFA+FB-,B=+AF-BFB-FA+", 3, 90.0);
        let mut cells: Vec<(i64, i64)> = hilbert
            .iter()
            .flat_map(|stroke| [stroke.from, stroke.to])
            .map(|(x, y)| (x.round() as i64, y.round() as i64))
            .collect();
        cells.sort_unstable();
        cells.dedup();
        assert_eq!(cells.len(), 64);
    }

    #[test]
    fn a_branch_returns_to_where_it_left() {
        let plant = drawn("F[+F]F[-F|F]fF", "", 0, 90.0);
        let ends: Vec<(i64, i64, u32)> = plant
            .iter()
            .map(|stroke| (stroke.to.0.round() as i64, stroke.to.1.round() as i64, stroke.branch))
            .collect();
        assert_eq!(ends, [(0, 1, 0), (-1, 1, 1), (0, 2, 0), (1, 2, 1), (0, 2, 1), (0, 4, 0)]);
    }

    #[test]
    fn the_rules_are_read_and_refused() {
        let rules = rules_of("X=X+YF+, Y=-FX-Y;Z=").unwrap();
        assert_eq!(rules[usize::from(b'Y')], Some(&b"-FX-Y"[..]));
        assert_eq!(rules[usize::from(b'Z')], Some(&b""[..]));
        for wrong in ["F", "FF=F", "F=F,F=G", "F=F=G", "=F", "F=Fé"] {
            assert!(rules_of(wrong).is_err(), "{wrong}");
        }
        assert!(written("F+[G]", LONGEST_AXIOM, "=").is_ok());
        assert!(written("F=G", LONGEST_AXIOM, "=").is_err());
        assert!(written("", LONGEST_AXIOM, "=").is_err());
    }

    #[test]
    fn a_drawing_stays_within_its_limits() {
        let rules = rules_of("F=FFFF").unwrap();
        assert_eq!(depth(b"F", &rules, 30), 10);
        let rules = rules_of("X=XX").unwrap();
        assert_eq!(depth(b"X", &rules, 30), 23);
        for kind in &KINDS {
            assert!(written(kind.axiom, LONGEST_AXIOM, "=").is_ok(), "{}", kind.axiom);
            assert!(rules_of(kind.rules).is_ok(), "{}", kind.rules);
        }
    }

    #[test]
    fn it_renders_to_its_golden_hash() {
        let hash = golden(&preset("l_system-dragon").unwrap());
        assert_eq!(hash, 0x0138_18d8_9974_580b, "got {hash:#018x}");
    }
}
