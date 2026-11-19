struct Ply {
    color: vec4<f32>, // the color, the opacity
    to: vec4<f32>,    // the color the first one runs to across the light, 1 when there is one
    span: vec4<f32>,  // the first of its shapes, how many
};

struct Frame {
    size: vec4<u32>,   // pixels across and down, sheets, the rows of the shapes
    view: vec4<f32>,   // the width of a pixel, half the picture in pixels (x, y), the longer half of the picture
    light: vec4<f32>,  // where the light comes from (x, y), how far from a shape its distance is exact
    look: vec4<f32>,   // rim, rim_width, bevel, gradient
    shade: vec4<f32>,  // shadow, shadow_offset, shadow_blur
    ground: vec4<f32>, // the background
    plies: array<Ply, 48>,
};

struct Batch {
    region: vec4<u32>, // left, top, pixels across, pixels down
    stride: vec4<u32>, // the side of the block a sample fills
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> spent: array<atomic<u32>>;
@group(0) @binding(3) var levels: texture_storage_2d<rgba16float, write>;
// Seven rows a shape: the center and the turn (cos, sin); the reach, the extent (x, y), the kind; two rows of
// the kind's own numbers, a wave's swells from the second on. A box that is not turned has in its fifth row the
// diagonal of its corner where the processor's 64-bit numbers tie: the pixels across less the pixels down, its
// first row of pixels, how many, and where its rows begin after the shapes. Those hold, two bits a pixel and
// twelve pixels a number, what the processor finds there: 1 no rise, 2 a rise across only, 3 a rise down only.
@group(0) @binding(4) var<storage, read> shapes: array<vec4<f32>>;

const ROWS: u32 = 7u;
const STEP: f32 = 1e-3;
const BOX: u32 = 0u;
const PETAL: u32 = 1u;
const RING: u32 = 2u;
const WAVE: u32 = 3u;
// What sheets.rs counts a scene's work in: a shape too far to matter, one whose distance is worked out, a swell.
const NEAR: u32 = 1u;
const WHOLE: u32 = 4u;
const SWELL: u32 = 3u;

var<private> work: u32;

struct Nearest {
    gap: f32,
    shape: u32,
};

fn turned(v: vec2<f32>, turn: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(v.x * turn.x + v.y * turn.y, v.y * turn.x - v.x * turn.y);
}

fn long(v: vec2<f32>) -> f32 {
    return sqrt(v.x * v.x + v.y * v.y);
}

// fractique-core's sheets.rs, Solid::distance.
fn away(shape: u32, at: vec2<f32>) -> f32 {
    let base = shape * ROWS;
    let place = shapes[base];
    let bounds = shapes[base + 1u];
    let apart = at - place.xy;
    let kind = u32(bounds.w);
    if (kind != WAVE) {
        let off = long(apart) - bounds.x;
        if (off > frame.light.z) {
            work += NEAR;
            return off;
        }
    }
    work += WHOLE;
    let p = turned(apart, place.zw);
    let a = shapes[base + 2u];
    let b = shapes[base + 3u];
    switch kind {
        case 0u: {
            let q = abs(p) - a.xy;
            for (var corner = 0u; corner < 4u; corner++) {
                let radius = b[corner];
                if (radius <= 0.0) {
                    continue;
                }
                let side = vec2<f32>(
                    select(-1.0, 1.0, corner == 1u || corner == 2u),
                    select(-1.0, 1.0, corner >= 2u),
                );
                let v = (p - side * (a.xy - radius)) * side;
                if (v.x > 0.0 && v.y > 0.0) {
                    let round = long(v) - radius;
                    if (q.x <= 0.0 && q.y <= 0.0) {
                        return max(round, max(q.x, q.y));
                    }
                    return round;
                }
            }
            return long(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
        }
        case 1u: {
            let m = abs(p);
            if ((m.y - a.z) * a.y > m.x * a.z) {
                return long(vec2<f32>(m.x, m.y - a.z));
            }
            return long(vec2<f32>(m.x + a.y, m.y)) - a.x;
        }
        case 2u: {
            let across = abs(p.x);
            if (a.z > 0.5 && b.y * across > b.x * -p.y) {
                return long(vec2<f32>(across - b.x * a.x, -p.y - b.y * a.x)) - a.y;
            }
            return abs(long(vec2<f32>(across, p.y)) - a.x) - a.y;
        }
        default: {
            var height = 0.0;
            var slope = 0.0;
            let swells = min(u32(a.y), 4u);
            for (var index = 0u; index < swells; index++) {
                let swell = shapes[base + 3u + index];
                let phase = p.x * swell.y + swell.z;
                height += swell.x * sin(phase);
                slope += swell.x * swell.y * cos(phase);
                work += SWELL;
            }
            let gap = (height - p.y) / sqrt(1.0 + slope * slope);
            if (a.x > 0.0) {
                return abs(gap) - a.x;
            }
            return gap;
        }
    }
}

// Solid::across.
fn across(shape: u32, at: vec2<f32>) -> f32 {
    let place = shapes[shape * ROWS];
    let bounds = shapes[shape * ROWS + 1u];
    if (u32(bounds.w) == WAVE) {
        return 0.0;
    }
    let p = turned(at - place.xy, place.zw);
    let light = turned(frame.light.xy, place.zw);
    return clamp(p.x / bounds.y, -1.0, 1.0) * light.x + clamp(p.y / bounds.z, -1.0, 1.0) * light.y;
}

fn tied(shape: u32, pixel: vec2<f32>) -> u32 {
    let mark = shapes[shape * ROWS + 4u];
    if (batch.stride.x != 1u || u32(shapes[shape * ROWS + 1u].w) != BOX || mark.z < 0.5) {
        return 0u;
    }
    let at = vec2<i32>(floor(pixel));
    let down = at.y - i32(mark.y);
    if (at.x - at.y != i32(mark.x) || down < 0 || down >= i32(mark.z)) {
        return 0u;
    }
    let index = u32(down);
    let held = shapes[frame.size.w + u32(mark.w) + index / 48u][(index / 12u) % 4u];
    return (u32(held) >> (2u * (index % 12u))) & 3u;
}

fn nearest(first: u32, count: u32, at: vec2<f32>) -> Nearest {
    var best = Nearest(3.0e38, first);
    for (var index = 0u; index < count; index++) {
        let gap = away(first + index, at);
        if (gap < best.gap) {
            best = Nearest(gap, first + index);
        }
    }
    return best;
}

fn eased(value: f32) -> f32 {
    let t = clamp(value, 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// Frame::row, for one pixel.
fn shaded(pixel: vec2<f32>) -> vec3<f32> {
    let p = (pixel - frame.view.yz) * frame.view.x;
    let light = frame.light.xy;
    let slope = (p.x * light.x + p.y * light.y) / frame.view.w;
    var color = frame.ground.rgb;
    let plies = min(frame.size.z, 48u);
    for (var index = 0u; index < plies; index++) {
        let ply = frame.plies[index];
        let first = u32(ply.span.x);
        let count = min(u32(ply.span.y), 32u);
        let thrown = nearest(first, count, p + light * frame.shade.y).gap;
        color *= 1.0 - frame.shade.x * ply.color.a * eased(0.5 - thrown / (2.0 * frame.shade.z));

        let near = nearest(first, count, p);
        let cover = clamp(0.5 - near.gap / frame.view.x, 0.0, 1.0) * ply.color.a;
        if (cover <= 0.0) {
            continue;
        }
        var rise = vec2<f32>(
            away(near.shape, p + vec2<f32>(STEP, 0.0)) - near.gap,
            away(near.shape, p + vec2<f32>(0.0, STEP)) - near.gap,
        );
        switch tied(near.shape, pixel) {
            case 1u: {
                rise = vec2<f32>(0.0);
            }
            case 2u: {
                rise = vec2<f32>(-1.0, 0.0);
            }
            case 3u: {
                rise = vec2<f32>(0.0, -1.0);
            }
            default: {}
        }
        let steep = long(rise);
        var facing = 0.0;
        if (steep > 0.0) {
            facing = (rise.x * light.x + rise.y * light.y) / steep;
        }
        let lit = (1.0 + frame.look.z * across(near.shape, p)) * (1.0 + frame.look.w * slope);
        let rim = frame.look.x * exp(min(near.gap, 0.0) / frame.look.y) * (0.4 + 0.6 * max(facing, 0.0));
        var base = ply.color.rgb;
        if (ply.to.a > 0.5) {
            base += (ply.to.rgb - base) * clamp(0.5 - 0.5 * slope, 0.0, 1.0);
        }
        color += (base * lit + rim - color) * cover;
    }
    return color;
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
    let color = vec4<f32>(shaded(vec2<f32>(at) + 0.5 * f32(side)), 1.0);
    for (var j = 0u; j < side; j++) {
        for (var i = 0u; i < side; i++) {
            let to = at + vec2<u32>(i, j);
            if (inside(to)) {
                textureStore(levels, to, color);
            }
        }
    }
    atomicAdd(&spent[0], (work + 8u) >> 4u);
}
