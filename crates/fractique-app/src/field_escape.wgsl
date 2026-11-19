const ESCAPE: f32 = 65536.0;
const LONGEST: u32 = 0x40000000u;
// fractique-core's escape/mod.rs: the steps of a point at the very most.
const MOST_STEPS: u32 = 4000000u;

// fractique-core's escape/mod.rs. The derivatives are kept divided by the zoom over four, so that they stay
// within 32 bits at any depth: what they are held against is then in shorter sides of the picture already.
struct Coloring {
    density: f32,
    shift: f32,
    rim: f32,
    reach: f32,
    base: f32,
};

fn coloring() -> Coloring {
    return Coloring(frame.p[0].x, frame.p[0].y, frame.p[0].z, frame.p[0].w, frame.p[1].x);
}

// `far` and `slope` are the sizes of the orbit and of its derivative; `flat` tells that a derivative of
// nothing is one too small for 32 bits, and not one that was never there.
fn settled(c: Coloring, steps: u32, half_log: f32, far: f32, slope: f32, flat: bool) -> vec2<f32> {
    let count = max(f32(steps) + 1.0 - log(half_log) / c.base, 0.0);
    var light = 1.0;
    if (!(slope <= 0.0) || flat) {
        var distance = 0.0;
        if (slope <= 0.0) {
            distance = NEVER;
        } else if (slope < NEVER) {
            distance = far / slope * half_log;
        }
        if (distance < c.rim) {
            light = powered(distance / c.rim, 0.4);
        }
        if (c.reach > 0.0) {
            light /= 1.0 + powered(distance / c.reach, 1.2);
        }
    }
    return vec2<f32>(c.shift + c.density * log(1.0 + count), light);
}

fn outside(c: Coloring, steps: u32, far: f32, slope: f32) -> vec2<f32> {
    return settled(c, steps, log(far), far, slope, false);
}

fn mandelbrot(c: vec2<f32>) -> vec2<f32> {
    let left = c.x - 0.25;
    let cy2 = c.y * c.y;
    let q = left * left + cy2;
    if (!gauging && (q * (q + left) <= 0.25 * cy2 || (c.x + 1.0) * (c.x + 1.0) + cy2 <= 0.0625)) {
        return NOTHING;
    }
    let grows = frame.p[1].y;
    var z = vec2<f32>(0.0);
    var dz = vec2<f32>(0.0);
    var old = vec2<f32>(0.0);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let x2 = z.x * z.x;
        let y2 = z.y * z.y;
        if (x2 + y2 > ESCAPE && !gauging) {
            work += step;
            return outside(coloring(), step, size(z), size(dz));
        }
        dz = 2.0 * mul(z, dz) + vec2<f32>(grows, 0.0);
        z = vec2<f32>(x2 - y2 + c.x, 2.0 * z.x * z.y + c.y);
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}

// p[2]: the constant.
fn julia(at: vec2<f32>) -> vec2<f32> {
    let c = frame.p[2].xy;
    var z = at;
    var dz = vec2<f32>(frame.p[1].y, 0.0);
    var old = vec2<f32>(NEVER);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let x2 = z.x * z.x;
        let y2 = z.y * z.y;
        if (x2 + y2 > ESCAPE && !gauging) {
            work += step;
            return outside(coloring(), step, size(z), size(dz));
        }
        dz = 2.0 * mul(z, dz);
        z = vec2<f32>(x2 - y2 + c.x, 2.0 * z.x * z.y + c.y);
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}

// The maps of the Burning Ship and the Tricorn are not analytic: the derivatives of x and y by the real part
// (a) and by the imaginary part (b) of the point are four numbers.
fn folded(c: vec2<f32>, ship: bool) -> vec2<f32> {
    let grows = frame.p[1].y;
    var z = vec2<f32>(0.0);
    var a = vec2<f32>(0.0);
    var b = vec2<f32>(0.0);
    var old = vec2<f32>(0.0);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let x2 = z.x * z.x;
        let y2 = z.y * z.y;
        if (x2 + y2 > ESCAPE && !gauging) {
            work += step;
            let g = vec2<f32>(dot(z, a), dot(z, b));
            return outside(coloring(), step, size(z), size(g) / size(z));
        }
        var turn = vec2<f32>(-z.y, -z.x);
        if (ship) {
            turn = vec2<f32>(select(abs(z.y), -abs(z.y), z.x < 0.0), select(abs(z.x), -abs(z.x), z.y < 0.0));
        }
        let risen = 2.0 * vec2<f32>(z.x * a.x - z.y * a.y, z.x * b.x - z.y * b.y) + vec2<f32>(grows, 0.0);
        let lifted = 2.0 * vec2<f32>(dot(turn, a), dot(turn, b)) + vec2<f32>(0.0, grows);
        a = vec2<f32>(risen.x, lifted.x);
        b = vec2<f32>(risen.y, lifted.y);
        var twice = -2.0 * z.x * z.y;
        if (ship) {
            twice = 2.0 * abs(z.x * z.y);
        }
        z = vec2<f32>(x2 - y2 + c.x, twice + c.y);
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}

fn burning_ship(at: vec2<f32>) -> vec2<f32> {
    return folded(vec2<f32>(at.x, -at.y), true);
}

fn tricorn(at: vec2<f32>) -> vec2<f32> {
    return folded(at, false);
}

// p[2]: the power, and the multiplications that raise to the power less one, or a negative number.
fn multibrot(c: vec2<f32>) -> vec2<f32> {
    let grows = frame.p[1].y;
    let power = frame.p[2].x;
    let times = min(i32(frame.p[2].y), 6);
    var z = vec2<f32>(0.0);
    var dz = vec2<f32>(0.0);
    var old = vec2<f32>(0.0);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let r2 = dot(z, z);
        if (r2 > ESCAPE && !gauging) {
            work += step;
            return outside(coloring(), step, size(z), size(dz));
        }
        var raised = z;
        if (times >= 0) {
            for (var i = 0; i < times; i++) {
                raised = mul(raised, z);
            }
        } else if (r2 > 0.0 || gauging) {
            let angle = atan2(z.y, z.x) * (power - 1.0);
            raised = pow(r2, 0.5 * (power - 1.0)) * vec2<f32>(cos(angle), sin(angle));
        }
        dz = power * mul(raised, dz) + vec2<f32>(grows, 0.0);
        z = mul(raised, z) + c;
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}

// p[2]: 1 for the Julia form, the constant, the feedback. The picture stands on the real axis.
fn phoenix(at: vec2<f32>) -> vec2<f32> {
    let point = vec2<f32>(at.y, -at.x);
    let feedback = frame.p[2].w;
    var c = point;
    var z = vec2<f32>(0.0);
    var dz = vec2<f32>(0.0);
    var grows = frame.p[1].y;
    if (frame.p[2].x > 0.5) {
        c = frame.p[2].yz;
        z = point;
        dz = vec2<f32>(grows, 0.0);
        grows = 0.0;
    }
    var last = vec2<f32>(0.0);
    var last_dz = vec2<f32>(0.0);
    var old = vec4<f32>(NEVER);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let x2 = z.x * z.x;
        let y2 = z.y * z.y;
        if (x2 + y2 > ESCAPE && !gauging) {
            work += step;
            return outside(coloring(), step, size(z), size(dz));
        }
        let risen = 2.0 * mul(z, dz) + feedback * last_dz + vec2<f32>(grows, 0.0);
        last_dz = dz;
        dz = risen;
        let next = vec2<f32>(x2 - y2, 2.0 * z.x * z.y) + c + feedback * last;
        last = z;
        z = next;
        if (all(vec4<f32>(z, last) == old) && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = vec4<f32>(z, last);
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}

// p[2]: 1 for the second type; the logarithm of the power an orbit next to 1 is raised to.
// The orbit's distance from 1 is iterated beside the orbit: (z - 1) of the next step is the square or the cube
// of this one's over the denominator, times the ratio plus one, which 32 bits hold however near 1 it is.
fn magnet(c: vec2<f32>) -> vec2<f32> {
    let second = frame.p[2].x > 0.5;
    let grows = frame.p[1].y;
    let c1 = c - vec2<f32>(1.0, 0.0);
    let c2 = c - vec2<f32>(2.0, 0.0);
    let product = mul(c1, c2);
    let slope = 2.0 * c - vec2<f32>(3.0, 0.0);
    let far = select(4.0, 9.0, second);
    let near = select(4.0, 2.0, second) / max(dot(c, c), 1e-30);
    var z = vec2<f32>(0.0);
    var dz = vec2<f32>(0.0);
    var w = vec2<f32>(-1.0, 0.0);
    var old = vec2<f32>(NEVER);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, MOST_STEPS); step++) {
        let r2 = dot(z, z);
        if (r2 > ESCAPE && !gauging) {
            work += step;
            return outside(coloring(), step, size(z) / far, size(dz) / far);
        }
        let direct = z - vec2<f32>(1.0, 0.0);
        if (dot(direct, direct) > 1e-3) {
            w = direct;
        }
        if (dot(w, w) < 1e-8 && !gauging) {
            work += step;
            let close = max(size(w), 1e-18);
            var settling = coloring();
            settling.base = frame.p[2].y;
            let half_log = max(-log(close) - 0.5 * log(near), 1e-3);
            return settled(settling, step, half_log, close, size(dz), step > 0u);
        }
        let z2 = mul(z, z);
        var top = z2 + c1;
        var bottom = 2.0 * z + c2;
        var dbottom = 2.0 * dz + vec2<f32>(grows, 0.0);
        var lower = w;
        var order = 2.0;
        if (second) {
            top = mul(z2, z) + 3.0 * mul(c1, z) + product;
            bottom = 3.0 * z2 + 3.0 * mul(c2, z) + product + vec2<f32>(1.0, 0.0);
            dbottom = 6.0 * mul(z, dz) + 3.0 * mul(c2, dz) + (3.0 * z + slope) * grows;
            lower = mul(w, w);
            order = 3.0;
        }
        if (bottom.x == 0.0 && bottom.y == 0.0 && !gauging) {
            work += step;
            return NOTHING;
        }
        // The numerator less the denominator is (z - 1) to the order: the ratio less one, and its derivative,
        // are taken from it, where the difference of two numbers near each other would be rounding alone.
        let ratio = div(top, bottom);
        let gap = div(mul(lower, w), bottom);
        dz = 2.0 * mul(ratio, div(order * mul(lower, dz) - mul(gap, dbottom), bottom));
        z = mul(ratio, ratio);
        w = mul(gap, ratio + vec2<f32>(1.0, 0.0));
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, LONGEST) * 2u;
        }
    }
    work += frame.size.w;
    return ended(z);
}
