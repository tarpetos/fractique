struct Frame {
    size: vec4<u32>,  // pixels across and down, samples a side where the picture is not smooth, steps at most
    view: vec4<f32>,  // the center, the width of a pixel on the plane, 2.5 / gamma
    step: vec4<f32>,  // the plane's units a pixel to the right (x, y) and a pixel down (x, y)
    look: vec4<f32>,  // how far a pixel may differ from its neighbours and stay one sample
    p: array<vec4<f32>, 20>,
    palette: array<vec4<f32>, 256>,
};

struct Batch {
    region: vec4<u32>, // left, top, pixels across, pixels down
    stride: vec4<u32>, // the side of the block a sample fills
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> spent: array<atomic<u32>>;
@group(0) @binding(3) var levels: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var centers: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var before: texture_2d<f32>;

const NOTHING: vec2<f32> = vec2<f32>(0.0, -1.0);
const NEVER: f32 = 1e30;

var<private> work: u32;
// The samples of a batch that measures the card walk every step: nothing ends their loops early.
var<private> gauging: bool;

fn ended(z: vec2<f32>) -> vec2<f32> {
    if (gauging) {
        return vec2<f32>(z.x, -1.0);
    }
    return NOTHING;
}

fn powered(value: f32, power: f32) -> f32 {
    if (value > 0.0) {
        return pow(value, power);
    }
    return 0.0;
}

// The length of a vector whose square 32 bits may not hold; of one they do not hold itself, more than any.
fn size(v: vec2<f32>) -> f32 {
    let most = max(abs(v.x), abs(v.y));
    if (!(most < NEVER) || !(min(abs(v.x), abs(v.y)) < NEVER)) {
        return NEVER;
    }
    if (most > 0.0) {
        return most * length(v / most);
    }
    return 0.0;
}

fn mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn div(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let most = max(abs(b.x), abs(b.y));
    let by = b / most;
    return vec2<f32>(a.x * by.x + a.y * by.y, a.y * by.x - a.x * by.y) / (dot(by, by) * most);
}

// fractique-core's pixel.rs: a place on the palette and a light become a color.
fn colored(sample: vec2<f32>) -> vec4<f32> {
    if (!(sample.y >= 0.0)) {
        return vec4<f32>(0.0);
    }
    var light = sample.y;
    if (frame.view.w != 1.0) {
        light = powered(light, frame.view.w);
    }
    let place = (1.0 - abs(2.0 * fract(sample.x * 0.5) - 1.0)) * 255.0;
    let below = min(u32(place), 254u);
    let color = mix(frame.palette[below].rgb, frame.palette[below + 1u].rgb, place - f32(below));
    return vec4<f32>(color * light, min(light, 1.0));
}

fn placed(pixel: vec2<f32>) -> vec2<f32> {
    let middle = pixel - 0.5 * vec2<f32>(frame.size.xy);
    return frame.view.xy + middle.x * frame.step.xy + middle.y * frame.step.zw;
}

fn sampled(pixel: vec2<f32>) -> vec4<f32> {
    return colored(field(placed(pixel)));
}

fn inside(at: vec2<u32>) -> bool {
    return at.x < min(batch.region.x + batch.region.z, frame.size.x)
        && at.y < min(batch.region.y + batch.region.w, frame.size.y);
}

// One sample a block of pixels: the whole picture at once when the blocks are large, then pixel by pixel.
@compute @workgroup_size(8, 8)
fn first(@builtin(global_invocation_id) id: vec3<u32>) {
    let side = batch.stride.x;
    let at = batch.region.xy + id.xy * side;
    if (!inside(at)) {
        return;
    }
    work = 0u;
    let color = sampled(vec2<f32>(at) + 0.5 * f32(side));
    for (var j = 0u; j < side; j++) {
        for (var i = 0u; i < side; i++) {
            let to = at + vec2<u32>(i, j);
            if (inside(to)) {
                textureStore(centers, to, color);
                textureStore(levels, to, color);
            }
        }
    }
    atomicAdd(&spent[0], (work + 8u) >> 4u);
}

fn differs(at: vec2<u32>) -> bool {
    let own = textureLoad(before, at, 0);
    let last = vec2<i32>(frame.size.xy) - vec2<i32>(1);
    var even = true;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let beside = textureLoad(before, clamp(vec2<i32>(at) + vec2<i32>(i, j), vec2<i32>(0), last), 0);
            even = even && all(abs(beside - own) < vec4<f32>(frame.look.x));
        }
    }
    return !even;
}

// How many pixels of the region the next pass draws again: its work is known before it is sent.
@compute @workgroup_size(8, 8)
fn count(@builtin(global_invocation_id) id: vec3<u32>) {
    let at = batch.region.xy + id.xy;
    if (inside(at) && differs(at)) {
        atomicAdd(&spent[0], 1u);
    }
}

@compute @workgroup_size(8, 8)
fn gauge(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= batch.region.z || id.y >= batch.region.w) {
        return;
    }
    gauging = true;
    work = 0u;
    let sample = field(placed(vec2<f32>(id.xy % frame.size.xy) + 0.5));
    if (sample.x == 0.123456) {
        work += 16u;
    }
    atomicAdd(&spent[0], (work + 8u) >> 4u);
}

// A pixel that differs from a neighbour is drawn again from a grid of samples, as pixel.rs does.
@compute @workgroup_size(8, 8)
fn refine(@builtin(global_invocation_id) id: vec3<u32>) {
    let at = batch.region.xy + id.xy;
    if (!inside(at) || !differs(at)) {
        return;
    }
    work = 0u;
    let side = frame.size.z;
    var sum = vec4<f32>(0.0);
    for (var j = 0u; j < side; j++) {
        for (var i = 0u; i < side; i++) {
            sum += sampled(vec2<f32>(at) + (vec2<f32>(f32(i), f32(j)) + 0.5) / f32(side));
        }
    }
    textureStore(levels, at, sum / f32(side * side));
    atomicAdd(&spent[0], (work + 8u) >> 4u);
}
