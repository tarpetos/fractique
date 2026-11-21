struct Batch {
    steps: u32,
    fuse: u32,
    seed: vec2<u32>,
    layer: u32,
    walkers: u32,
    low: u32,    // the fewest and the most of its latest steps a particle keeps in a move
    high: u32,
    far: u32,    // below this many are kept in a far move
    cap: u32,    // the steps a particle remembers
    plots: u32,
    tiles: u32,  // the tiles of the picture across, and down from bit 16; 0 where the points are not spread
    window: f32, // the picture's size in the window's
    next: f32,   // and in the window after it
    travel: f32, // what turns cells into the units a move is told in
    last: f32,
};

@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> slots: array<u32>;
@group(0) @binding(4) var<storage, read_write> notes: array<atomic<u32>>;

// A particle's numbers: the walker it takes new steps from (its point, its clock, the generator of the particle),
// then the particle's point as cells and its color, then the seeds of its steps, the latest first.
const SLOT: u32 = 144u;
const POINT: u32 = 8u;
const SEEDS: u32 = 12u;
const MEMORY: u32 = 128u;
const STEP: u32 = 1u;
const FINISH: u32 = 2u;
const FAR_EVERY: u32 = 4u;
// The notes: 0 the points plotted or found, 1 those of them in the view (of a pilot: in the next window), 2 the
// particles inside the next window, 3 how often note 1 passed 32 bits, then for every number of kept steps the
// moves tried, the moves accepted and how far they went, then a mark for every particle (its tile, where the
// picture has tiles), the particle each takes the place of, and for every tile the power of two its points count.
const CARRY: u32 = 3u;
const TRIED: u32 = 4u;
const ACCEPTED: u32 = 133u;
const TRAVELLED: u32 = 262u;
const HEAD: u32 = 392u;
const FARTHEST: f32 = 4096.0;

var<private> own: vec2<u32>;
var<private> salt: u32;
var<private> clock: u32;
var<private> walker: vec3<f32>;
var<private> point: vec3<f32>;

fn seated(seed: u32, domain: u32) {
    let first = mixed(seed + domain * 0x632BE5ABu);
    state = vec2<u32>(first, mixed(first ^ seed) | 1u);
}

fn mine() -> u32 {
    state = own;
    let out = next();
    own = state;
    return out;
}

fn below(count: u32) -> u32 {
    return ((mine() >> 8u) * count) >> 24u;
}

fn seed_at(time: u32) -> u32 {
    return mixed(mixed(time) ^ salt);
}

fn tick() {
    clock = clock + 1u;
    seated(seed_at(clock), STEP);
    walker = stepped(walker).xyz;
}

fn inside(at: vec2<f32>, window: f32) -> bool {
    let half = flame.scale.zw * 0.5;
    let off = (at - half) * window;
    return abs(off.x) < half.x && abs(off.y) < half.y;
}

fn tile(at: vec2<f32>) -> u32 {
    let x = min(u32(max(at.x, 0.0)), flame.extent.x - 1u);
    let y = min(u32(max(at.y, 0.0)), flame.extent.y - 1u);
    let across = batch.tiles & 0xFFFFu;
    return (y * (batch.tiles >> 16u) / flame.extent.y) * across + x * across / flame.extent.x;
}

fn shift(at: vec2<f32>) -> u32 {
    if (batch.tiles == 0u) {
        return 0u;
    }
    return atomicLoad(&notes[HEAD + 2u * batch.walkers + tile(at)]);
}

// A move into a tile whose points count more is taken as many times less often.
fn admitted(out: u32, to: u32) -> bool {
    return to <= out || (mine() >> (32u - (to - out))) == 0u;
}

fn weighed(cell: u32, color: f32, times: u32) {
    let shade = min(u32(color * 255.0 + 0.5), 255u);
    let rgb = flame.palette[shade >> 2u][shade & 3u];
    let before = atomicAdd(&cells[cell], times);
    if (before < FULL) {
        atomicAdd(&cells[cell + 1u], (rgb & 255u) * times);
        atomicAdd(&cells[cell + 2u], ((rgb >> 8u) & 255u) * times);
        atomicAdd(&cells[cell + 3u], ((rgb >> 16u) & 255u) * times);
    }
}

// The point a particle would be at with the first `keep` of its seeds and the rest from its walker; w is 1 when
// that point is plotted and inside the window.
fn moved(base: u32, keep: u32, fresh: u32) -> vec4<f32> {
    var at = walker;
    for (var step = keep; step > 1u; step = step - 1u) {
        seated(slots[base + SEEDS + step - 1u], STEP);
        at = stepped(at).xyz;
        if (escaped) {
            return vec4<f32>(0.0);
        }
    }
    var finish = fresh;
    if (keep > 0u) {
        finish = slots[base + SEEDS];
    }
    var last = seed_at(clock);
    if (keep > 1u) {
        last = slots[base + SEEDS + 1u];
    }
    seated(last, STEP);
    let opacity = flame.xfs[chosen()].after.z;
    seated(finish, FINISH);
    if (opacity < 1.0 && unit() >= opacity) {
        return vec4<f32>(0.0);
    }
    let to = shown(at);
    if (abs(to.x) < 1e30 && abs(to.y) < 1e30 && inside(to.xy, batch.window)) {
        return vec4<f32>(to, 1.0);
    }
    return vec4<f32>(0.0);
}

fn taken(base: u32, keep: u32, fresh: u32) {
    if (keep == 0u) {
        slots[base + SEEDS] = fresh;
    }
    let first = max(keep, 1u);
    for (var step = first; step <= batch.cap; step = step + 1u) {
        slots[base + SEEDS + step] = seed_at(clock - (step - first));
    }
}

fn attempt(base: u32, keep: u32, told: bool) {
    let fresh = mine();
    tick();
    let to = moved(base, keep, fresh);
    if (told) {
        atomicAdd(&notes[TRIED + keep], 1u);
    }
    if (to.w > 0.0 && admitted(shift(point.xy), shift(to.xy))) {
        taken(base, keep, fresh);
        if (told) {
            let went = (abs(to.x - point.x) + abs(to.y - point.y)) * batch.travel;
            atomicAdd(&notes[ACCEPTED + keep], 1u);
            atomicAdd(&notes[TRAVELLED + keep], u32(min(went, FARTHEST)));
        }
        point = to.xyz;
    }
}

fn opened(index: u32) -> u32 {
    let base = index * SLOT;
    salt = mixed(index + mixed(batch.seed.x + mixed(batch.seed.y + mixed(batch.layer + 0xDEE9u))));
    walker = vec3<f32>(bitcast<f32>(slots[base]), bitcast<f32>(slots[base + 1u]), bitcast<f32>(slots[base + 2u]));
    clock = slots[base + 3u];
    own = vec2<u32>(slots[base + 4u], slots[base + 5u]);
    point = vec3<f32>(
        bitcast<f32>(slots[base + POINT]),
        bitcast<f32>(slots[base + POINT + 1u]),
        bitcast<f32>(slots[base + POINT + 2u]),
    );
    return base;
}

fn closed(base: u32) {
    slots[base] = bitcast<u32>(walker.x);
    slots[base + 1u] = bitcast<u32>(walker.y);
    slots[base + 2u] = bitcast<u32>(walker.z);
    slots[base + 3u] = clock;
    slots[base + 4u] = own.x;
    slots[base + 5u] = own.y;
    slots[base + POINT] = bitcast<u32>(point.x);
    slots[base + POINT + 1u] = bitcast<u32>(point.y);
    slots[base + POINT + 2u] = bitcast<u32>(point.z);
}

// Every particle looks for a point of the flame inside the first window, and tells whether it found one.
@compute @workgroup_size(64)
fn pilot(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= batch.walkers) {
        return;
    }
    let base = opened(index);
    own = vec2<u32>(mixed(index ^ salt), mixed(index + mixed(salt ^ 0x9E3779B9u)) | 1u);
    let x = f32(mine() >> 8u) * (2.0 / 16777216.0) - 1.0;
    let y = f32(mine() >> 8u) * (2.0 / 16777216.0) - 1.0;
    walker = vec3<f32>(x, y, f32(mine() >> 8u) * (1.0 / 16777216.0));
    clock = 0u;
    for (var i = 0u; i < MEMORY; i = i + 1u) {
        tick();
    }
    var found = 0u;
    var hits = 0u;
    var nearer = 0u;
    for (var i = 0u; i < batch.steps; i = i + 1u) {
        let fresh = mine();
        tick();
        let to = moved(base, 0u, fresh);
        if (to.w > 0.0) {
            hits = hits + 1u;
            if (inside(to.xy, batch.next)) {
                nearer = nearer + 1u;
            }
            if (found == 0u) {
                found = 1u;
                taken(base, 0u, fresh);
                point = to.xyz;
            }
        }
    }
    closed(base);
    atomicStore(&notes[HEAD + index], found);
    atomicAdd(&notes[0], hits);
    atomicAdd(&notes[1], nearer);
}

// A particle that fell out of the window becomes a copy of one that stayed.
@compute @workgroup_size(64)
fn refill(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= batch.walkers) {
        return;
    }
    let other = atomicLoad(&notes[HEAD + batch.walkers + index]);
    if (other == index || other >= batch.walkers) {
        return;
    }
    for (var i = POINT; i < SLOT; i = i + 1u) {
        slots[index * SLOT + i] = slots[other * SLOT + i];
    }
}

// Moves every particle about inside the window, and plots it after each move when the window is the picture.
@compute @workgroup_size(64)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= batch.walkers) {
        return;
    }
    let base = opened(index);
    let across = u32(flame.scale.z);
    let down = u32(flame.scale.w);
    let rim = flame.extent.zw;
    var seen = 0u;
    for (var i = 0u; i < batch.steps; i = i + 1u) {
        if (batch.far > 0u && below(FAR_EVERY) == 0u) {
            attempt(base, below(batch.far), false);
        }
        attempt(base, batch.low + below(batch.high - batch.low + 1u), batch.plots == 0u);
        if (batch.plots == 1u) {
            let x = min(u32(max(point.x, 0.0)), across - 1u);
            let y = min(u32(max(point.y, 0.0)), down - 1u);
            let times = 1u << shift(point.xy);
            weighed((y * flame.extent.x + x) * 4u, point.z, times);
            if (x >= rim.x && x < across - rim.x && y >= rim.y && y < down - rim.y) {
                seen = seen + times;
            }
        }
    }
    closed(base);
    if (batch.plots == 1u) {
        atomicAdd(&notes[0], batch.steps);
        if (atomicAdd(&notes[1], seen) > 0xFFFFFFFFu - seen) {
            atomicAdd(&notes[CARRY], 1u);
        }
    }
}

// Tells which particles are inside the next window, and in a picture with tiles the tile of each.
@compute @workgroup_size(64)
fn mark(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= batch.walkers) {
        return;
    }
    let base = index * SLOT;
    let at = vec2<f32>(bitcast<f32>(slots[base + POINT]), bitcast<f32>(slots[base + POINT + 1u]));
    var stays = 0u;
    if (inside(at, batch.next)) {
        stays = 1u;
    }
    atomicAdd(&notes[2], stays);
    if (batch.tiles != 0u) {
        stays = tile(at);
    }
    atomicStore(&notes[HEAD + index], stays);
}
