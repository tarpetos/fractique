struct Xf {
    affine: vec4<f32>, // a, b, c, d
    shift: vec4<f32>,  // e, f, color, color speed
    post: vec4<f32>,   // a, b, c, d
    after: vec4<f32>,  // e, f, opacity, cumulative weight
    range: vec4<u32>,  // first variation, how many, 1 with a post map
};

struct Var {
    kind: u32,
    weight: f32,
    spare: vec2<f32>,
    params: vec4<f32>,
};

struct Flame {
    counts: vec4<u32>, // transforms, copies of the symmetry, 1 when mirrored, 1 with a final transform
    view: vec4<f32>,   // center, cosine and sine of the rotation
    scale: vec4<f32>,  // cells a unit across, cells a unit up, cells across, cells down
    extent: vec4<u32>, // cells across, cells down
    total: vec4<f32>,  // the sum of the weights
    xfs: array<Xf, 65>,
    vars: array<Var, 256>,
    palette: array<vec4<u32>, 64>,
    turns: array<vec4<f32>, 32>,
};

struct Batch {
    steps: u32,
    fuse: u32,
    seed: vec2<u32>,
    layer: u32,
    walkers: u32,
};

struct Walker {
    at: vec4<f32>,
    random: vec4<u32>,
};

@group(0) @binding(0) var<uniform> flame: Flame;
@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> walkers: array<Walker>;
@group(0) @binding(3) var<storage, read_write> cells: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> landed: array<atomic<u32>>;

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530717959;
const EPS: f32 = 1e-10;
const ESCAPED: f32 = 1e10;
// 255 a hit keeps a color sum of this many hits inside 32 bits.
const FULL: u32 = 16000000u;

var<private> state: vec2<u32>;

fn mixed(value: u32) -> u32 {
    let s = value * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn turned(value: u32, by: u32) -> u32 {
    return (value << by) | (value >> (32u - by));
}

// xoroshiro64**
fn next() -> u32 {
    let s0 = state.x;
    var s1 = state.y;
    let out = turned(s0 * 0x9E3779BBu, 5u) * 5u;
    s1 = s1 ^ s0;
    state.x = turned(s0, 26u) ^ s1 ^ (s1 << 9u);
    state.y = turned(s1, 13u);
    return out;
}

fn unit() -> f32 {
    return f32(next() >> 8u) * (1.0 / 16777216.0);
}

fn coin() -> bool {
    return (next() >> 31u) == 1u;
}

fn angle(y: f32, x: f32) -> f32 {
    if (x == 0.0 && y == 0.0) {
        return 0.0;
    }
    return atan2(y, x);
}

fn folded(v: f32, size: f32) -> f32 {
    if (size == 0.0) {
        return v;
    }
    return (2.0 * floor(v / size) + 1.0) * size - v;
}

fn varied(kind: u32, x: f32, y: f32, p: vec4<f32>, weight: f32, affine: vec4<f32>, shift: vec2<f32>) -> vec2<f32> {
    let r2 = x * x + y * y;
    let r = sqrt(r2);
    switch kind {
        case 0u: {
            return vec2<f32>(x, y);
        }
        case 1u: {
            return vec2<f32>(sin(x), sin(y));
        }
        case 2u: {
            let k = 1.0 / (r2 + EPS);
            return vec2<f32>(x * k, y * k);
        }
        case 3u: {
            let s = sin(r2);
            let c = cos(r2);
            return vec2<f32>(x * s - y * c, x * c + y * s);
        }
        case 4u: {
            let k = 1.0 / (r + EPS);
            return vec2<f32>((x - y) * (x + y) * k, 2.0 * x * y * k);
        }
        case 5u: {
            return vec2<f32>(angle(x, y) / PI, r - 1.0);
        }
        case 6u: {
            let t = angle(x, y);
            return vec2<f32>(r * sin(t + r), r * cos(t - r));
        }
        case 7u: {
            let t = angle(x, y);
            return vec2<f32>(r * sin(t * r), -r * cos(t * r));
        }
        case 8u: {
            let t = angle(x, y) / PI;
            return vec2<f32>(t * sin(PI * r), t * cos(PI * r));
        }
        case 9u: {
            let t = angle(x, y);
            let rr = r + EPS;
            return vec2<f32>((cos(t) + sin(rr)) / rr, (sin(t) - cos(rr)) / rr);
        }
        case 10u: {
            let t = angle(x, y);
            let rr = r + EPS;
            return vec2<f32>(sin(t) / rr, rr * cos(t));
        }
        case 11u: {
            let t = angle(x, y);
            return vec2<f32>(sin(t) * cos(r), cos(t) * sin(r));
        }
        case 12u: {
            let t = angle(x, y);
            let p0 = sin(t + r);
            let p1 = cos(t - r);
            let m0 = p0 * p0 * p0;
            let m1 = p1 * p1 * p1;
            return vec2<f32>(r * (m0 + m1), r * (m0 - m1));
        }
        case 13u: {
            let rr = sqrt(r);
            var t = angle(x, y) * 0.5;
            if (coin()) {
                t = t + PI;
            }
            return vec2<f32>(rr * cos(t), rr * sin(t));
        }
        case 14u: {
            var out = vec2<f32>(x, y);
            if (x < 0.0) {
                out.x = 2.0 * x;
            }
            if (y < 0.0) {
                out.y = 0.5 * y;
            }
            return out;
        }
        case 15u: {
            let k = 2.0 / (r + 1.0);
            return vec2<f32>(k * y, k * x);
        }
        case 16u: {
            let k = 2.0 / (r + 1.0);
            return vec2<f32>(k * x, k * y);
        }
        case 17u: {
            let k = exp(x - 1.0);
            return vec2<f32>(k * cos(PI * y), k * sin(PI * y));
        }
        case 18u: {
            let t = angle(x, y);
            let s = sin(t);
            let c = cos(t);
            let k = pow(r, s);
            return vec2<f32>(k * c, k * s);
        }
        case 19u: {
            return vec2<f32>(cos(PI * x) * cosh(y), -sin(PI * x) * sinh(y));
        }
        case 20u: {
            let k = 4.0 / (r2 + 4.0);
            return vec2<f32>(k * x, k * y);
        }
        case 21u: {
            return vec2<f32>(sin(x), y);
        }
        case 22u: {
            return vec2<f32>(sin(x) / (cos(y) + EPS), tan(y));
        }
        case 23u: {
            let s = x * x - y * y;
            let k = sqrt(1.0 / (s * s + EPS));
            return vec2<f32>(k * x, k * y);
        }
        case 24u: {
            let rr = unit();
            let a = unit() * TAU;
            return vec2<f32>(rr * cos(a), rr * sin(a));
        }
        case 25u: {
            let rr = unit() + unit() + unit() + unit() - 2.0;
            let a = unit() * TAU;
            return vec2<f32>(rr * cos(a), rr * sin(a));
        }
        case 26u: {
            let branch = trunc(abs(p.x) * unit());
            let a = (angle(y, x) + TAU * branch) / p.x;
            let rr = pow(r2 + EPS, p.y / p.x * 0.5);
            return vec2<f32>(rr * cos(a), rr * sin(a));
        }
        case 27u: {
            let re = 1.0 + p.x * x + p.y * (x * x - y * y);
            let im = p.x * y + 2.0 * p.y * x * y;
            let k = 1.0 / (re * re + im * im + EPS);
            return vec2<f32>((x * re + y * im) * k, (y * re - x * im) * k);
        }
        case 28u: {
            return vec2<f32>(
                x + affine.y * sin(y / (affine.z * affine.z + EPS)),
                y + shift.x * sin(x / (shift.y * shift.y + EPS)),
            );
        }
        case 29u: {
            return vec2<f32>(x + affine.z * sin(tan(3.0 * y)), y + shift.y * sin(tan(3.0 * x)));
        }
        case 30u: {
            let c2 = affine.z * affine.z + EPS;
            let t = angle(x, y);
            let k = (r + c2) % (2.0 * c2) - c2 + r * (1.0 - c2);
            return vec2<f32>(k * cos(t), k * sin(t));
        }
        case 31u: {
            let half = PI * (affine.z * affine.z + EPS) * 0.5;
            let t = angle(x, y);
            var a = t + half;
            if ((t + shift.y) % (2.0 * half) > half) {
                a = t - half;
            }
            return vec2<f32>(r * cos(a), r * sin(a));
        }
        case 32u: {
            let t = angle(x, y);
            let k = r * (p.x + (p.y - p.x) * 0.5 * (sin(p.z * t) + 1.0));
            return vec2<f32>(k * cos(t), k * sin(t));
        }
        case 33u: {
            return vec2<f32>(sin(p.x * y) - cos(p.y * x), sin(p.z * x) - cos(p.w * y));
        }
        case 34u: {
            let wedge = PI * p.x * p.x + EPS;
            let t = angle(x, y);
            let within = t + p.y - wedge * trunc(2.0 * t * p.y / wedge);
            var a = t + wedge * 0.5;
            if (within > wedge * 0.5) {
                a = t - wedge * 0.5;
            }
            return vec2<f32>(r * sin(a), r * cos(a));
        }
        case 35u: {
            let v = p.x * p.x + EPS;
            let t = angle(x, y);
            let k = r - 2.0 * v * trunc((r + v) / (2.0 * v)) + r * (1.0 - v);
            return vec2<f32>(k * sin(t), k * cos(t));
        }
        case 36u: {
            let a = p.x * PI * 0.5;
            let k = p.y / (p.y - y * sin(a));
            return vec2<f32>(k * x, k * y * cos(a));
        }
        case 37u: {
            let rr = unit();
            let a = unit() * TAU;
            return vec2<f32>(rr * x * cos(a), rr * y * sin(a));
        }
        case 38u: {
            let branch = trunc(abs(p.x) * unit());
            var side = angle(y, x);
            if (branch % 2.0 != 0.0) {
                side = -side;
            }
            let a = (side + TAU * branch) / p.x;
            let rr = pow(r2 + EPS, p.y / p.x * 0.5);
            return vec2<f32>(rr * cos(a), rr * sin(a));
        }
        case 39u: {
            let spread = weight * (unit() + unit() + unit() + unit() - 2.0);
            if (weight == 0.0) {
                return vec2<f32>(0.0, 0.0);
            }
            let a = p.x * PI * 0.5;
            let to = angle(y, x) + spread * sin(a);
            let zoom = spread * cos(a) - 1.0;
            return vec2<f32>((r * cos(to) + zoom * x) / weight, (r * sin(to) + zoom * y) / weight);
        }
        case 40u: {
            let slice = trunc(unit() * p.x + 0.5);
            let a = p.y + TAU * (slice + unit() * p.z) / p.x;
            let rr = unit();
            return vec2<f32>(rr * cos(a), rr * sin(a));
        }
        case 41u: {
            let side = TAU / p.x;
            let phi = angle(y, x);
            var within = phi - side * floor(phi / side);
            if (within > side * 0.5) {
                within = within - side;
            }
            let k = (p.w * (1.0 / cos(within) - 1.0) + p.z) / pow(r2 + EPS, p.y * 0.5);
            return vec2<f32>(k * x, k * y);
        }
        case 42u: {
            return vec2<f32>(folded(x, p.x), folded(y, p.y));
        }
        case 43u: {
            let a = unit() * PI * weight;
            let s = sin(a);
            return vec2<f32>(s, s * s / cos(a));
        }
        case 44u: {
            let first = unit() - 0.5;
            return vec2<f32>(first, unit() - 0.5);
        }
        case 45u: {
            let k = weight * tan(unit() * PI * weight) / (r2 + EPS);
            return vec2<f32>(k * cos(x), k * sin(y));
        }
        case 46u: {
            let a = unit() * weight * r;
            let s = sin(a);
            let c = cos(a);
            return vec2<f32>(x * (c + s), x * (c - s));
        }
        case 47u: {
            let c = cos(weight * r);
            var up = 1.0 / c - 1.0;
            if (c < 0.0) {
                up = 1.0 / c + 1.0;
            }
            return vec2<f32>(x, up);
        }
        case 48u: {
            let a = unit() * weight * r;
            let s = sin(a);
            let c = cos(a);
            var k = -30.0 + c;
            if (s != 0.0) {
                k = log(s * s) * 0.4342944819 + c;
            }
            return vec2<f32>(x * k, x * (k - PI * s));
        }
        default: {
            return vec2<f32>(x, y);
        }
    }
}

fn applied(which: u32, at: vec2<f32>) -> vec2<f32> {
    let affine = flame.xfs[which].affine;
    let shift = flame.xfs[which].shift.xy;
    let range = flame.xfs[which].range;
    let x = affine.x * at.x + affine.y * at.y + affine.z;
    let y = affine.w * at.x + shift.x * at.y + shift.y;
    var out = vec2<f32>(0.0, 0.0);
    for (var i = range.x; i < range.x + range.y; i = i + 1u) {
        let weight = flame.vars[i].weight;
        out = out + weight * varied(flame.vars[i].kind, x, y, flame.vars[i].params, weight, affine, shift);
    }
    if (range.z == 1u) {
        let post = flame.xfs[which].post;
        let after = flame.xfs[which].after.xy;
        out = vec2<f32>(post.x * out.x + post.y * out.y + post.z, post.w * out.x + after.x * out.y + after.y);
    }
    return out;
}

fn chosen() -> u32 {
    let pick = unit() * flame.total.x;
    let last = flame.counts.x - 1u;
    var which = last;
    for (var i = 0u; i < last; i = i + 1u) {
        if (pick < flame.xfs[i].after.w) {
            which = i;
            break;
        }
    }
    return which;
}

// One step of the chaos game: the point with its color, and in w the transform that made it.
fn stepped(start: vec3<f32>) -> vec4<f32> {
    let which = chosen();
    let to = applied(which, start.xy);
    let look = flame.xfs[which].shift.zw;
    let color = start.z + (look.x - start.z) * look.y;
    if (abs(to.x) < ESCAPED && abs(to.y) < ESCAPED) {
        return vec4<f32>(to, color, f32(which));
    }
    let x = unit() * 2.0 - 1.0;
    return vec4<f32>(x, unit() * 2.0 - 1.0, color, f32(which));
}

@compute @workgroup_size(64)
fn walk(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= batch.walkers) {
        return;
    }
    var at: vec3<f32>;
    if (batch.fuse > 0u) {
        let salt = mixed(batch.seed.x + mixed(batch.seed.y + mixed(batch.layer + 0x5EEDu)));
        state = vec2<u32>(mixed(index ^ salt), mixed(index + mixed(salt ^ 0x9E3779B9u)) | 1u);
        let x = unit() * 2.0 - 1.0;
        let y = unit() * 2.0 - 1.0;
        at = vec3<f32>(x, y, unit());
        for (var i = 0u; i < batch.fuse; i = i + 1u) {
            at = stepped(at).xyz;
        }
    } else {
        at = walkers[index].at.xyz;
        state = walkers[index].random.xy;
    }

    let across = flame.scale.z;
    let down = flame.scale.w;
    var hits = 0u;
    for (var i = 0u; i < batch.steps; i = i + 1u) {
        let moved = stepped(at);
        at = moved.xyz;
        let opacity = flame.xfs[u32(moved.w)].after.z;
        var seen = true;
        if (opacity < 1.0) {
            seen = unit() < opacity;
        }
        if (seen) {
            var point = at.xy;
            var color = at.z;
            if (flame.counts.w == 1u) {
                point = applied(64u, point);
                let look = flame.xfs[64u].shift.zw;
                color = color + (look.x - color) * look.y;
            }
            if (flame.counts.y > 1u) {
                let copy = ((next() >> 8u) * flame.counts.y) >> 24u;
                let pair = flame.turns[copy >> 1u];
                var turn = pair.xy;
                if ((copy & 1u) == 1u) {
                    turn = pair.zw;
                }
                point = vec2<f32>(point.x * turn.x - point.y * turn.y, point.x * turn.y + point.y * turn.x);
            }
            if (flame.counts.z == 1u) {
                if (coin()) {
                    point.x = -point.x;
                }
            }
            let d = point - flame.view.xy;
            let px = across * 0.5 + (d.x * flame.view.z + d.y * flame.view.w) * flame.scale.x;
            let py = down * 0.5 - (d.y * flame.view.z - d.x * flame.view.w) * flame.scale.y;
            if (px >= 0.0 && px < across && py >= 0.0 && py < down) {
                let shade = min(u32(color * 255.0 + 0.5), 255u);
                let rgb = flame.palette[shade >> 2u][shade & 3u];
                let cell = (u32(py) * flame.extent.x + u32(px)) * 4u;
                let before = atomicAdd(&cells[cell], 1u);
                if (before < FULL) {
                    atomicAdd(&cells[cell + 1u], rgb & 255u);
                    atomicAdd(&cells[cell + 2u], (rgb >> 8u) & 255u);
                    atomicAdd(&cells[cell + 3u], (rgb >> 16u) & 255u);
                }
                hits = hits + 1u;
            }
        }
    }
    walkers[index].at = vec4<f32>(at, 0.0);
    walkers[index].random = vec4<u32>(state, 0u, 0u);
    atomicAdd(&landed[0], hits);
}
