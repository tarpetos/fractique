struct Orbit {
    counts: vec4<u32>,  // kind, coloring, 1 when fitted + 2 when blended, steps before the first point
    view: vec4<f32>,    // center, cosine and sine of the rotation
    scale: vec4<f32>,   // cells a unit across, cells a unit up, cells across, cells down
    extent: vec4<u32>,  // cells across, cells down, the slot after the walkers where the tables begin
    span: vec4<u32>,    // of a tracked orbit: its steps, the steps between tracked states, the states, the steps of a stream
    tables: vec4<u32>,  // where each table begins, in numbers
    sizes: vec4<u32>,   // the numbers of each table
    middle: vec4<f32>,  // the middle of the picture, 1 / its unit
    ranks: vec4<f32>,   // lowest pace, steps of its table a unit, lowest height, steps of its table a unit
    params: array<vec4<f32>, 2>,
    numbers: array<vec4<f32>, 4>,
    palette: array<vec4<u32>, 64>,
};

struct Batch {
    steps: u32,
    fuse: u32,
    seed: vec2<u32>,
    layer: u32,
    walkers: u32,
    budget: u32,
    spare: u32,
};

// A walker of an attractor: its state, then the random state, the step of its stream and the steps to skip.
// After the walkers the same slots hold the tables, four numbers each.
struct Walker {
    at: vec4<f32>,
    random: vec4<u32>,
};

@group(0) @binding(0) var<uniform> orbit: Orbit;
@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> walkers: array<Walker>;
@group(0) @binding(3) var<storage, read_write> cells: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> landed: array<atomic<u32>>;

const PI: f32 = 3.14159265358979;
const ESCAPED: f32 = 1e9;
const FULL: u32 = 16000000u;
const TIME_STEP: f32 = 0.005;

// The kinds, as orbit.rs numbers them: 1 clifford, 2 de_jong, 4 ikeda.
const LORENZ: u32 = 3u;

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

fn seeded(index: u32) {
    let salt = mixed(batch.seed.x + mixed(batch.seed.y + mixed(batch.layer + 0x0B17u)));
    state = vec2<u32>(mixed(index ^ salt), mixed(index + mixed(salt ^ 0x9E3779B9u)) | 1u);
}

fn table(which: u32, at: u32) -> f32 {
    let place = orbit.tables[which] + at;
    return walkers[orbit.extent.z + (place >> 2u)].at[place & 3u];
}

fn param(at: u32) -> f32 {
    return orbit.params[at >> 2u][at & 3u];
}

fn number(at: u32) -> f32 {
    return orbit.numbers[at >> 2u][at & 3u];
}

fn plot(point: vec2<f32>, place: f32) -> u32 {
    let d = point - orbit.view.xy;
    let px = orbit.scale.z * 0.5 + (d.x * orbit.view.z + d.y * orbit.view.w) * orbit.scale.x;
    let py = orbit.scale.w * 0.5 - (d.y * orbit.view.z - d.x * orbit.view.w) * orbit.scale.y;
    if (px >= 0.0 && px < orbit.scale.z && py >= 0.0 && py < orbit.scale.w) {
        var shade = 0u;
        if (place > 0.0) {
            shade = min(u32(place * 255.0 + 0.5), 255u);
        }
        let rgb = orbit.palette[shade >> 2u][shade & 3u];
        let cell = (u32(py) * orbit.extent.x + u32(px)) * 4u;
        let before = atomicAdd(&cells[cell], 1u);
        if (before < FULL) {
            atomicAdd(&cells[cell + 1u], rgb & 255u);
            atomicAdd(&cells[cell + 2u], (rgb >> 8u) & 255u);
            atomicAdd(&cells[cell + 3u], (rgb >> 16u) & 255u);
        }
        return 1u;
    }
    return 0u;
}

fn start() -> vec3<f32> {
    let x = unit() - 0.5;
    return vec3<f32>(x, unit() - 0.5, 0.0);
}

fn flow(at: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(param(0u) * (at.y - at.x), at.x * (param(1u) - at.z) - at.y, at.x * at.y - param(2u) * at.z);
}

fn stepped(kind: u32, at: vec3<f32>) -> vec3<f32> {
    switch kind {
        case 1u: {
            let a = param(0u);
            let b = param(1u);
            return vec3<f32>(sin(a * at.y) + param(2u) * cos(a * at.x), sin(b * at.x) + param(3u) * cos(b * at.y), 0.0);
        }
        case 2u: {
            return vec3<f32>(
                sin(param(0u) * at.y) - cos(param(1u) * at.x),
                sin(param(2u) * at.x) - cos(param(3u) * at.y),
                0.0,
            );
        }
        case 3u: {
            let k1 = flow(at);
            let k2 = flow(at + k1 * (TIME_STEP * 0.5));
            let k3 = flow(at + k2 * (TIME_STEP * 0.5));
            let k4 = flow(at + k3 * TIME_STEP);
            return at + (k1 + 2.0 * (k2 + k3) + k4) * (TIME_STEP / 6.0);
        }
        case 4u: {
            let t = param(1u) - param(2u) / (1.0 + dot(at.xy, at.xy));
            let turn = vec2<f32>(cos(t), sin(t));
            return vec3<f32>(1.0 + param(0u) * (at.x * turn.x - at.y * turn.y), param(0u) * (at.x * turn.y + at.y * turn.x), 0.0);
        }
        default: {
            return at;
        }
    }
}

fn pictured(kind: u32, at: vec3<f32>) -> vec2<f32> {
    if (kind == LORENZ) {
        let across = at.x * number(0u) - at.y * number(1u);
        let depth = at.x * number(1u) + at.y * number(0u);
        return vec2<f32>(across, (at.z - (param(1u) - 1.0)) * number(2u) + depth * number(3u));
    }
    return at.xy;
}

fn ranked(which: u32, value: f32, low: f32, per_step: f32) -> f32 {
    let size = orbit.sizes[which];
    if (size < 2u) {
        return 0.0;
    }
    var place = 0.0;
    if (value > low) {
        place = min((value - low) * per_step, f32(size));
    }
    let step = min(u32(place), size - 2u);
    let under = table(which, step);
    return under + (table(which, step + 1u) - under) * min(place - f32(step), 1.0);
}

fn placed(kind: u32, before: vec3<f32>, after: vec3<f32>, was: vec2<f32>, now: vec2<f32>) -> f32 {
    switch orbit.counts.y {
        case 0u: {
            var pace = length(now - was);
            if (kind == LORENZ) {
                pace = length(after - before);
            }
            return ranked(0u, pace, orbit.ranks.x, orbit.ranks.y);
        }
        case 1u: {
            let d = now - was;
            if (d.x == 0.0 && d.y == 0.0) {
                return 0.0;
            }
            return abs(atan2(d.y, d.x)) / PI;
        }
        case 2u: {
            var height = length(now - orbit.middle.xy);
            if (kind == LORENZ) {
                height = after.z;
            }
            return ranked(1u, height, orbit.ranks.z, orbit.ranks.w);
        }
        default: {
            return 0.5 + 0.5 * sin(param(0u) * before.y);
        }
    }
}

fn traced(index: u32) {
    let kind = orbit.counts.x;
    let span = orbit.span.x;
    let every = orbit.span.y;
    let stream = orbit.span.w;
    var at: vec3<f32>;
    // The processor walks 64 streams, each the one orbit again and again: a walker of a tracked orbit is at a
    // step of such a stream and begins at the tracked state before it.
    var time = 0u;
    var skip = 0u;
    if (batch.fuse > 0u) {
        seeded(index);
        if (every > 0u) {
            time = min(u32((f32(index) + unit()) / f32(batch.walkers) * f32(stream)), stream - 1u);
            time = time - (time % span) % every;
        } else {
            at = start();
            skip = orbit.counts.w;
        }
    } else {
        at = walkers[index].at.xyz;
        state = walkers[index].random.xy;
        time = walkers[index].random.z;
        skip = walkers[index].random.w;
    }

    var plotted = 0u;
    var spent = 0u;
    var hits = 0u;
    loop {
        if (plotted >= batch.steps || spent >= batch.budget) {
            break;
        }
        spent = spent + 1u;
        if (skip > 0u) {
            at = stepped(kind, at);
            skip = skip - 1u;
            if (!(abs(at.x) < ESCAPED && abs(at.y) < ESCAPED)) {
                at = start();
                skip = orbit.counts.w;
            }
            continue;
        }
        if (every > 0u && (time % span) % every == 0u) {
            let place = ((time % span) / every) * 3u;
            at = vec3<f32>(table(2u, place), table(2u, place + 1u), table(2u, place + 2u));
        }
        let after = stepped(kind, at);
        let was = pictured(kind, at);
        var now = pictured(kind, after);
        let place = placed(kind, at, after, was, now);
        if ((orbit.counts.z & 2u) != 0u) {
            now = was + (now - was) * unit();
        }
        let gone = !(abs(now.x) < ESCAPED && abs(now.y) < ESCAPED);
        if ((orbit.counts.z & 1u) != 0u) {
            now = (now - orbit.middle.xy) * orbit.middle.z;
        }
        hits = hits + plot(now, place);
        plotted = plotted + 1u;
        at = after;
        if (every > 0u) {
            time = time + 1u;
            if (gone) {
                time = (time / span + 1u) * span;
            }
            if (time >= stream) {
                time = 0u;
            }
        } else if (gone) {
            at = start();
            skip = orbit.counts.w;
        }
    }
    walkers[index].at = vec4<f32>(at, 0.0);
    walkers[index].random = vec4<u32>(state, time, skip);
    atomicAdd(&landed[0], hits);
    atomicAdd(&landed[1], plotted);
}

@compute @workgroup_size(64)
fn walk(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= batch.walkers) {
        return;
    }
    traced(id.x);
}
