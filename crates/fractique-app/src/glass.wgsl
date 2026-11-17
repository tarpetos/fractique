// Draws the flame as the window's background, and glass panels that refract and blur it.
// Both read the same two textures: the flame, and a blurred copy made once per render.

struct Uniforms {
    screen: vec4<f32>, // width, height (pixels), unused, unused
    rect: vec4<f32>,   // panel: left, top, right, bottom (pixels)
    shape: vec4<f32>,  // corner radius, bezel width, shadow reach (pixels), shadow strength
    optics: vec4<f32>, // refraction (pixels), blur (0..1), tint (0..1), rim light (0..1)
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

fn look(p: vec2<f32>) -> vec3<f32> {
    let uv = p / u.screen.xy;
    return mix(textureSample(flame, smp, uv).rgb, textureSample(soft, smp, uv).rgb, u.optics.y);
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
    let red = look(p + shift * 1.12).r;
    let green = look(p + shift).g;
    let blue = look(p + shift * 0.88).b;
    var color = vec3<f32>(red, green, blue);

    // Glass body: clear glass brightens what is behind it; tint smokes it so text on it stays readable.
    color = color * mix(1.2, 0.5, u.optics.z) + vec3<f32>(mix(0.015, 0.1, u.optics.z));
    // A faint sheen, strongest at the top of the pane.
    let down = clamp((p.y - u.rect.y) / max(u.rect.w - u.rect.y, 1.0), 0.0, 1.0);
    color = color + vec3<f32>(0.045 * (1.0 - down) * (1.0 - down) * u.optics.w);

    // Light comes from the top left: it catches the rim there and, fainter, on the opposite side.
    let light = normalize(vec2<f32>(-0.6, -0.8));
    let facing = dot(outward, light);
    let glint = pow(rim, 5.0) * (0.75 * pow(max(facing, 0.0), 1.5) + 0.3 * pow(max(-facing, 0.0), 2.0) + 0.08);
    let edge_line = smoothstep(-2.2, -0.6, distance) * 0.22;
    color = color + vec3<f32>((glint + edge_line) * u.optics.w);

    let inside = clamp(0.5 - distance, 0.0, 1.0);
    let reach = max(u.shape.z, 1.0);
    let shadow = u.shape.w * exp(-max(distance, 0.0) / (reach * 0.4)) * (1.0 - inside);
    // Premultiplied alpha, as egui's own pass uses.
    return vec4<f32>(color * inside, inside + shadow * (1.0 - inside));
}
