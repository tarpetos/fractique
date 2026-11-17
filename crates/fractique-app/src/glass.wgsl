// Draws the flame as the window's background, and glass panels that refract and blur it.
// Both read the same two textures: the flame, and a blurred copy made once per render.

struct Uniforms {
    screen: vec4<f32>, // width, height (pixels), unused, unused
    rect: vec4<f32>,   // panel: left, top, right, bottom (pixels)
    shape: vec4<f32>,  // corner radius, bezel width, shadow reach (pixels), shadow strength
    optics: vec4<f32>, // refraction (pixels), blur (0..1), opacity (0..1), depth (0..1)
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var flame: texture_2d<f32>;
@group(0) @binding(2) var soft: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // One triangle that covers the viewport egui sets to the callback's rectangle.
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_background(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(flame, smp, pos.xy / u.screen.xy).rgb;
    return vec4<f32>(color, 1.0);
}

fn look(p: vec2<f32>, blur: f32) -> vec3<f32> {
    let uv = p / u.screen.xy;
    return mix(textureSample(flame, smp, uv).rgb, textureSample(soft, smp, uv).rgb, blur);
}

@fragment
fn fs_glass(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = pos.xy;
    let center = (u.rect.xy + u.rect.zw) * 0.5;
    let half_size = (u.rect.zw - u.rect.xy) * 0.5;
    let radius = min(u.shape.x, min(half_size.x, half_size.y));

    // Signed distance to the rounded rectangle (negative inside) and the outward direction at this pixel.
    let from_center = p - center;
    let q = abs(from_center) - half_size + vec2<f32>(radius);
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    var outward = vec2<f32>(0.0, 1.0);
    if (q.x > 0.0 && q.y > 0.0) {
        outward = normalize(q);
    } else if (q.x > q.y) {
        outward = vec2<f32>(1.0, 0.0);
    }
    outward = outward * vec2<f32>(select(-1.0, 1.0, from_center.x >= 0.0), select(-1.0, 1.0, from_center.y >= 0.0));

    // 0 on the flat middle of the pane, 1 at its rim: the bezel is where the glass curves and bends light.
    let rim = clamp(1.0 + distance / max(u.shape.y, 1.0), 0.0, 1.0);
    let bend = rim * rim * rim;

    // The curved rim works as a lens: it shows the background from further in, squeezed, with a little dispersion.
    let shift = -outward * bend * u.optics.x;
    // The rim stays sharper than the middle, so the bending is seen and not blurred away.
    let blur = u.optics.y * (1.0 - 0.6 * bend);
    let red = look(p + shift * 1.1, blur).r;
    let green = look(p + shift, blur).g;
    let blue = look(p + shift * 0.9, blur).b;
    var color = vec3<f32>(red, green, blue);

    // Glass body: opacity 0 is clear glass, 1 is dark smoked glass that hides most of the flame.
    let smoke = vec3<f32>(0.035, 0.04, 0.06);
    color = mix(color * 1.08, smoke, u.optics.z * 0.85);

    // What makes the pane look thick: its rim is curved, so the side turned to the light (top left) shows the
    // background brighter and the far side darker. It scales the background instead of painting white over it.
    let facing = dot(outward, normalize(vec2<f32>(-0.6, -0.8)));
    let curve = rim * rim;
    color = color * (1.0 + 0.6 * curve * facing * u.optics.w) + vec3<f32>(0.03 * curve * max(facing, 0.0) * u.optics.w);

    // A thin soft glint on the very edge, on the lit corner and fainter on the opposite one.
    let edge = smoothstep(-2.6, -0.7, distance);
    let glint = edge * (0.3 * pow(max(facing, 0.0), 2.0) + 0.13 * pow(max(-facing, 0.0), 2.0) + 0.035);
    color = color + vec3<f32>(glint * u.optics.w);

    let inside = clamp(0.5 - distance, 0.0, 1.0);
    let reach = max(u.shape.z, 1.0);
    let shadow = u.shape.w * exp(-max(distance, 0.0) / (reach * 0.4)) * (1.0 - inside);
    // Premultiplied alpha, as egui's own pass uses.
    return vec4<f32>(color * inside, inside + shadow * (1.0 - inside));
}
