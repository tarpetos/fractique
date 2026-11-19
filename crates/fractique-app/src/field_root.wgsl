// fractique-core's root/mod.rs: the part of a step by which an orbit came under the distance that ends it.
fn part(before: f32, now: f32, near: f32, squares: bool) -> f32 {
    if (!(before < NEVER)) {
        return 1.0;
    }
    let b = -log(before);
    let n = -log(max(now, 1e-37));
    let r = -log(near);
    var share = 1.0;
    if (!squares) {
        share = (r - b) / (n - b);
    } else if (b > 0.0) {
        share = log(r / b) / log(n / b);
    }
    return clamp(share, 0.0, 1.0);
}

fn shaded(count: f32, shade: f32, half: f32) -> f32 {
    let dark = shade * count / half;
    return 1.0 / (1.0 + dark * dark);
}

fn pair(first: u32, index: u32) -> vec2<f32> {
    let both = frame.p[first + index / 2u];
    return select(both.xy, both.zw, (index & 1u) == 1u);
}

// p[0]: the degree, 1 where the distance is squared at every step, the shade, the steps to half the light.
// p[1]: the relaxation, the color density and shift. p[2..]: the polynomial from the highest power down.
// p[9..]: the roots.
fn newton(at: vec2<f32>) -> vec2<f32> {
    let degree = min(u32(frame.p[0].x), 12u);
    let relaxation = frame.p[1].xy;
    var z = at;
    var last = vec2<f32>(NEVER);
    for (var step = 0u; step < min(frame.size.w, 4000000u); step++) {
        var nearest = 0u;
        var apart = NEVER;
        for (var index = 0u; index < degree; index++) {
            let off = z - pair(9u, index);
            let away = dot(off, off);
            if (away < apart) {
                nearest = index;
                apart = away;
            }
        }
        if (apart < 1e-6 && !gauging) {
            work += step * degree;
            let was = last - pair(9u, nearest);
            var before = NEVER;
            if (last.x < NEVER) {
                before = dot(was, was);
            }
            let count = max(f32(step) - 1.0 + part(before, apart, 1e-6, frame.p[0].y > 0.5), 0.0);
            let place = fract(frame.p[1].w + f32(nearest) / f32(degree));
            return vec2<f32>(
                place + 0.05 * frame.p[1].z * log(1.0 + count),
                shaded(count, frame.p[0].z, frame.p[0].w),
            );
        }
        // Far from the origin the polynomial and its slope are taken in powers of 1 / z, which 32 bits hold.
        let far = dot(z, z) > 4.0;
        var value = pair(2u, select(0u, degree, far));
        var slope = vec2<f32>(0.0);
        if (far) {
            let back = div(vec2<f32>(1.0, 0.0), z);
            for (var term = 1u; term <= degree; term++) {
                let own = pair(2u, degree - term);
                slope = mul(slope, back) + f32(term) * own;
                value = mul(value, back) + own;
            }
            value = mul(value, z);
        } else {
            for (var term = 1u; term <= degree; term++) {
                slope = mul(slope, z) + value;
                value = mul(value, z) + pair(2u, term);
            }
        }
        if (slope.x == 0.0 && slope.y == 0.0 && !gauging) {
            work += step * degree;
            return NOTHING;
        }
        last = z;
        z -= mul(relaxation, div(value, slope));
    }
    work += frame.size.w * degree;
    return ended(z);
}

// p[0]: 1 for the Julia form, the degree, the relaxation over the degree, the shade.
// p[1]: the constant, the color density and shift. p[2]: where the orbit of the first form starts.
fn nova(at: vec2<f32>) -> vec2<f32> {
    let degree = min(u32(frame.p[0].y), 8u);
    var c = at;
    var z = frame.p[2].xy;
    if (frame.p[0].x > 0.5) {
        c = frame.p[1].xy;
        z = at;
    }
    var before = NEVER;
    var old = vec2<f32>(NEVER);
    var lap = 0u;
    var period = 8u;
    for (var step = 0u; step < min(frame.size.w, 4000000u); step++) {
        // (zⁿ - 1) / zⁿ⁻¹ is z less one over zⁿ⁻¹, and far from the origin it is z: zⁿ would not fit in 32 bits.
        var off = z;
        if (!(dot(z, z) > 1e8)) {
            var low = z;
            for (var index = 2u; index < degree; index++) {
                low = mul(low, z);
            }
            if (low.x == 0.0 && low.y == 0.0 && !gauging) {
                work += step * degree;
                return NOTHING;
            }
            off = z - div(vec2<f32>(1.0, 0.0), low);
        }
        let by = c - frame.p[0].z * off;
        let moved = dot(by, by);
        if (moved < 1e-10 && !gauging) {
            work += step * degree;
            let count = f32(step) + part(before, moved, 1e-10, false);
            return vec2<f32>(frame.p[1].w + frame.p[1].z * log(1.0 + count), shaded(count, frame.p[0].w, 40.0));
        }
        before = moved;
        z += by;
        if (z.x == old.x && z.y == old.y && !gauging) {
            work += step * degree;
            return NOTHING;
        }
        lap++;
        if (lap == period) {
            old = z;
            lap = 0u;
            period = min(period, 0x40000000u) * 2u;
        }
    }
    work += frame.size.w * degree;
    return ended(z);
}
