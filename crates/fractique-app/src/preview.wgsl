struct Uniforms {
    tone: vec4<f32>,  // brightness, 1 / gamma, vibrancy, highlight
    blend: vec4<f32>, // opacity, glow strength, taps a side, 1 when a blur keeps its weight inside the picture
    reach: vec4<f32>, // source texels a target pixel (x, y), or the direction of a blur; sigma of a blur (texels);
                      // 1 for levels that are a color already
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var levels: texture_2d<f32>;
@group(0) @binding(2) var image: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn raised(value: vec3<f32>, power: f32) -> vec3<f32> {
    return select(pow(max(value, vec3<f32>(1e-12)), vec3<f32>(power)), vec3<f32>(0.0), value <= vec3<f32>(0.0));
}

// The tone mapping of fractique-core's tone.rs, for one pixel.
fn developed(at: vec2<i32>) -> vec4<f32> {
    let cell = textureLoad(levels, at, 0);
    if (u.reach.w > 0.5) {
        return vec4<f32>(cell.rgb * u.tone.x, cell.a);
    }
    let level = u.tone.x * cell.a;
    let capped = min(level, 1.0);
    let alpha = raised(vec3<f32>(capped), u.tone.y).x;
    let burn = min(max(level - 1.0, 0.0) * u.tone.w, 1.0);
    let vivid = cell.rgb * alpha;
    let flat = raised(cell.rgb * capped, u.tone.y);
    let value = flat + (vivid - flat) * u.tone.z;
    return vec4<f32>(value + (vec3<f32>(1.0) - value) * burn, alpha);
}

@fragment
fn fs_layer(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    var color = developed(vec2<i32>(pos.xy));
    let covered = vec2<f32>(textureDimensions(image)) * u.reach.xy;
    color = color + textureSampleLevel(image, smp, pos.xy / covered, 0.0) * u.blend.y;
    color.a = min(color.a, 1.0);
    return clamp(color * u.blend.x, vec4<f32>(0.0), vec4<f32>(1.0));
}

@fragment
fn fs_toned(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let taps = i32(u.blend.z);
    let limit = vec2<i32>(textureDimensions(levels)) - vec2<i32>(1);
    let corner = floor(pos.xy) * u.reach.xy;
    var sum = vec4<f32>(0.0);
    var count = 0.0;
    for (var j = 0; j < taps; j++) {
        for (var i = 0; i < taps; i++) {
            let at = vec2<i32>(corner + (vec2<f32>(f32(i), f32(j)) + vec2<f32>(0.5)) * u.reach.xy / f32(taps));
            if (at.x <= limit.x && at.y <= limit.y) {
                sum = sum + developed(at);
                count = count + 1.0;
            }
        }
    }
    return sum / max(count, 1.0);
}

@fragment
fn fs_shrink(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let taps = i32(u.blend.z);
    let size = vec2<f32>(textureDimensions(image));
    let corner = floor(pos.xy) * u.reach.xy;
    var sum = vec4<f32>(0.0);
    for (var j = 0; j < taps; j++) {
        for (var i = 0; i < taps; i++) {
            let at = corner + (vec2<f32>(f32(i), f32(j)) + vec2<f32>(0.5)) * u.reach.xy / f32(taps);
            sum = sum + textureSampleLevel(image, smp, clamp(at / size, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
        }
    }
    return sum / f32(taps * taps);
}

@fragment
fn fs_blur(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(image));
    let sigma = max(u.reach.z, 0.01);
    let reach = min(i32(ceil(sigma * 3.0)), 12);
    var sum = vec4<f32>(0.0);
    var whole = 0.0;
    var within = 0.0;
    for (var i = -reach; i <= reach; i++) {
        let weight = exp(-f32(i * i) / (2.0 * sigma * sigma));
        let at = (pos.xy + u.reach.xy * f32(i)) / size;
        let inside = step(vec2<f32>(0.0), at) * step(at, vec2<f32>(1.0));
        sum = sum + textureSampleLevel(image, smp, at, 0.0) * weight * inside.x * inside.y;
        whole = whole + weight;
        within = within + weight * inside.x * inside.y;
    }
    return sum / mix(whole, max(within, 1e-6), u.blend.w);
}
