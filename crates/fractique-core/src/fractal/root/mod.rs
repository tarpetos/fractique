use super::escape::{COLOR_DENSITY, COLOR_SHIFT, Complex, div, mul, norm};
use super::{Param, Tone};
use crate::math::ln;

super::family! {
    newton,
    nova,
}

const ITERATIONS: Param = Param::whole("iterations", 200.0, 8.0, 10_000.0).log();
const RELAXATION: Param = Param::real("relaxation", 1.0, 0.1, 1.9);
const SHADE: Param = Param::real("shade", 0.5, 0.0, 1.0);

const TONE: Tone = Tone {
    brightness: 1.0,
    gamma: 2.5,
    vibrancy: 1.0,
    highlight: 0.0,
    glow_radius: 0.004,
    glow_strength: 0.06,
    smoothing: 1.0,
};

// The part of a step by which an orbit came under `near`: `before` and `now` are the squared distances
// around it. Where the distance is squared at every step the logarithm doubles, elsewhere it grows evenly.
#[inline]
fn part(before: f64, now: f64, near: f64, squares: bool) -> f64 {
    if !before.is_finite() {
        return 1.0;
    }
    let (before, now, near) = (-ln(before), -ln(now.max(1e-300)), -ln(near));
    let part = if !squares {
        (near - before) / (now - before)
    } else if before <= 0.0 {
        1.0
    } else {
        ln(near / before) / ln(now / before)
    };
    part.clamp(0.0, 1.0)
}

// The light of a point that took `count` steps: darker the more steps, and half at `half` over the shade.
#[inline]
fn light(count: f64, shade: f64, half: f64) -> f64 {
    let dark = shade * count / half;
    1.0 / (1.0 + dark * dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_part_of_a_step_joins_one_step_to_the_next() {
        for squares in [false, true] {
            assert_eq!(part(0.3, 1e-6, 1e-6, squares), 1.0);
            assert!(part(1.000_001e-6, 1e-9, 1e-6, squares) < 1e-6);
            let (early, late) = (part(1e-3, 1e-7, 1e-6, squares), part(1e-3, 1e-9, 1e-6, squares));
            assert!(0.0 < late && late < early && early < 1.0);
        }
        assert_eq!(part(f64::INFINITY, 1e-9, 1e-6, false), 1.0);
        assert_eq!(part(4.0, 1e-9, 1e-6, true), 1.0);
    }
}
