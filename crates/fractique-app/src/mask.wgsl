struct Uniforms {
    shape: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let half = uniforms.shape.xy * 0.5;
    let radius = uniforms.shape.z;
    let from_arc = max(abs(at.xy - half) - (half - vec2<f32>(radius)), vec2<f32>(0.0));
    let outside = clamp(length(from_arc) - radius + 0.5, 0.0, 1.0);
    return vec4<f32>(0.0, 0.0, 0.0, outside);
}
