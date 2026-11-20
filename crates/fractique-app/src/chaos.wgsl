struct Batch {
    steps: u32,
    fuse: u32,
    seed: vec2<u32>,
    layer: u32,
    walkers: u32,
    budget: u32,
    idle: u32,
};

struct Walker {
    at: vec4<f32>,
    random: vec4<u32>,
};

@group(0) @binding(1) var<uniform> batch: Batch;
@group(0) @binding(2) var<storage, read_write> walkers: array<Walker>;
@group(0) @binding(4) var<storage, read_write> landed: array<atomic<u32>>;

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
        // The rest of the steps a walker takes before it plots: the first unit took the others.
        for (var i = 0u; i < batch.idle; i = i + 1u) {
            at = stepped(at).xyz;
        }
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
            let point = shown(at);
            if (point.x >= 0.0 && point.x < across && point.y >= 0.0 && point.y < down) {
                plotted((u32(point.y) * flame.extent.x + u32(point.x)) * 4u, point.z);
                hits = hits + 1u;
            }
        }
    }
    walkers[index].at = vec4<f32>(at, 0.0);
    walkers[index].random = vec4<u32>(state, 0u, 0u);
    atomicAdd(&landed[0], hits);
}
