struct Uniforms {
    screen: vec4<f32>, // width, height (pixels), unused, unused
    rect: vec4<f32>,   // panel: left, top, right, bottom (pixels)
    shape: vec4<f32>,  // corner radius, bezel width, shadow reach (pixels), shadow strength
    optics: vec4<f32>, // refraction (pixels), blur (0..1), opacity (0..1), depth (0..1)
    view: vec4<f32>,   // the picture point at the middle of the screen (0..1, 0..1), magnification, picture aspect / screen aspect
    past: vec4<f32>,   // the same for the picture before this one
    blend: vec4<f32>,  // the share of this picture over the one before (0..1), the blur over both (0..1), the pane's opacity, 1 while the picture stands for another view
};

fn picture(p: vec2<f32>, view: vec4<f32>) -> vec2<f32> {
    var from_middle = p / u.screen.xy - vec2<f32>(0.5);
    if (view.w > 1.0) {
        from_middle.y = from_middle.y * view.w;
    } else {
        from_middle.x = from_middle.x / view.w;
    }
    return from_middle / view.z + view.xy;
}

const AROUND: vec3<f32> = vec3<f32>(0.012, 0.014, 0.045);

fn on_picture(uv: vec2<f32>) -> f32 {
    let inside = step(vec2<f32>(0.0), uv) * step(uv, vec2<f32>(1.0));
    return inside.x * inside.y;
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var flame: texture_2d<f32>;
@group(0) @binding(2) var soft: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;
@group(0) @binding(4) var past_flame: texture_2d<f32>;
@group(0) @binding(5) var past_soft: texture_2d<f32>;

// Beside a picture there is the window's own dark. While another picture is on its way and this one stands moved
// or made smaller, its edge would be a line across the window: what is beside it is then the blurred picture,
// turned over at its edge, fading into the dark over a third of the picture.
fn beside(uv: vec2<f32>, turned: vec3<f32>) -> vec3<f32> {
    let off = max(abs(uv - vec2<f32>(0.5)) - vec2<f32>(0.5), vec2<f32>(0.0));
    let near = 1.0 - smoothstep(0.0, 0.33, max(off.x, off.y));
    return mix(AROUND, turned, near * max(min(u.blend.y * 2.0, 1.0), u.blend.w));
}

fn turned_over(uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(1.0) - abs(vec2<f32>(1.0) - abs(uv));
}

// The picture at a point of the window, blurred by a share: the one before it too while it fades into this one.
fn look(p: vec2<f32>, blur: f32) -> vec3<f32> {
    let share = max(blur, u.blend.y);
    let uv = picture(p, u.view);
    let sharp = textureSample(flame, smp, uv).rgb;
    let blurred = textureSample(soft, smp, uv).rgb;
    let turned = textureSample(soft, smp, turned_over(uv)).rgb;
    let now = mix(beside(uv, turned), mix(sharp, blurred, share), on_picture(uv));
    if (u.blend.x >= 1.0) {
        return now;
    }
    let old_uv = picture(p, u.past);
    let old_sharp = textureSample(past_flame, smp, old_uv).rgb;
    let old_blurred = textureSample(past_soft, smp, old_uv).rgb;
    let old_turned = textureSample(past_soft, smp, turned_over(old_uv)).rgb;
    let before = mix(beside(old_uv, old_turned), mix(old_sharp, old_blurred, share), on_picture(old_uv));
    return mix(before, now, u.blend.x);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_background(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(look(pos.xy, 0.0), 1.0);
}

@fragment
fn fs_glass(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = pos.xy;
    let center = (u.rect.xy + u.rect.zw) * 0.5;
    let half_size = (u.rect.zw - u.rect.xy) * 0.5;
    let radius = min(u.shape.x, min(half_size.x, half_size.y));

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

    let rim = clamp(1.0 + distance / max(u.shape.y, 1.0), 0.0, 1.0);
    let bend = rim * rim * rim;

    let shift = -outward * bend * u.optics.x;
    let blur = u.optics.y * (1.0 - 0.6 * bend);
    let red = look(p + shift * 1.1, blur).r;
    let green = look(p + shift, blur).g;
    let blue = look(p + shift * 0.9, blur).b;
    var color = vec3<f32>(red, green, blue);

    let smoke = vec3<f32>(0.035, 0.04, 0.06);
    color = mix(color * 1.08, smoke, u.optics.z * 0.85);

    let facing = dot(outward, normalize(vec2<f32>(-0.6, -0.8)));
    let curve = rim * rim;
    color = color * (1.0 + 0.6 * curve * facing * u.optics.w) + vec3<f32>(0.03 * curve * max(facing, 0.0) * u.optics.w);

    let edge = smoothstep(-2.6, -0.7, distance);
    let glint = edge * (0.3 * pow(max(facing, 0.0), 2.0) + 0.13 * pow(max(-facing, 0.0), 2.0) + 0.035);
    color = color + vec3<f32>(glint * u.optics.w);

    let within = clamp(0.5 - distance, 0.0, 1.0);
    let reach = max(u.shape.z, 1.0);
    let shadow = u.shape.w * exp(-max(distance, 0.0) / (reach * 0.4)) * (1.0 - within);
    // Premultiplied alpha, as egui's own pass uses.
    return vec4<f32>(color * within, within + shadow * (1.0 - within)) * u.blend.z;
}
