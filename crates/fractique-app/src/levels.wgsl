struct Look {
    size: vec4<u32>,   // cells across, cells down, tiles across, tiles down
    scale: vec4<f32>,  // what turns a count into a density, the first row of the strip that is drawn, the logarithm's scale
    kernels: array<vec4<f32>, 20>, // reach, 1 / (2 sigma²), 1 / the sum of the weights
    widths: array<vec4<u32>, 1024>, // the kernel of a cell by its hits, up to 4095
    grid: vec4<u32>,   // the tiles of a deep view across and down; 0 where every hit counts once
    shifts: array<vec4<u32>, 144>, // the power of two a hit of a tile counts
};

@group(0) @binding(0) var<uniform> look: Look;
@group(0) @binding(1) var<storage, read> cells: array<u32>;
@group(0) @binding(2) var<storage, read_write> tiles: array<u32>;
@group(0) @binding(3) var levels: texture_storage_2d<rgba16float, write>;

const TILE: u32 = 16u;
const FULL: u32 = 16000000u;

fn level_of(count: u32) -> u32 {
    if (count >= 4096u) {
        return 0u;
    }
    return look.widths[count >> 2u][count & 3u];
}

fn tiled(column: f32, row: f32) -> f32 {
    let at = min(vec2<u32>(max(vec2<f32>(column, row), vec2<f32>(0.0))), look.grid.xy - vec2<u32>(1u));
    let tile = at.y * look.grid.x + at.x;
    return f32(look.shifts[tile >> 2u][tile & 3u]);
}

// The kernel of a cell goes by its hits: its count over the shift mixed between the middles of the tiles around it.
fn level_at(count: u32, x: u32, y: u32) -> u32 {
    if (look.grid.x == 0u) {
        return level_of(count);
    }
    let among = (vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5)) / vec2<f32>(look.size.xy) * vec2<f32>(look.grid.xy)
        - vec2<f32>(0.5);
    let corner = floor(among);
    let mix = among - corner;
    let first = tiled(corner.x, corner.y);
    let lower = first + (tiled(corner.x + 1.0, corner.y) - first) * mix.x;
    let second = tiled(corner.x, corner.y + 1.0);
    let upper = second + (tiled(corner.x + 1.0, corner.y + 1.0) - second) * mix.x;
    return level_of(max(count >> u32(lower + (upper - lower) * mix.y + 0.5), 1u));
}

@compute @workgroup_size(8, 8)
fn survey(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= look.size.z || id.y >= look.size.w) {
        return;
    }
    let corner = id.xy * TILE;
    let limit = min(corner + vec2<u32>(TILE), look.size.xy);
    var most = 0u;
    for (var y = corner.y; y < limit.y; y = y + 1u) {
        for (var x = corner.x; x < limit.x; x = x + 1u) {
            let count = cells[(y * look.size.x + x) * 4u];
            if (count > 0u) {
                most = max(most, u32(look.kernels[level_at(count, x, y)].x) + 1u);
            }
        }
    }
    tiles[id.y * look.size.z + id.x] = most;
}

@compute @workgroup_size(8, 8)
fn resolve(@builtin(global_invocation_id) id: vec3<u32>) {
    // The levels are drawn a strip of rows at a time: scale.y is the strip's first row.
    let at = vec2<i32>(id.xy) + vec2<i32>(0, i32(look.scale.y));
    if (at.x >= i32(look.size.x) || at.y >= i32(look.size.y)) {
        return;
    }
    let tile = at / i32(TILE);
    let last = vec2<i32>(look.size.zw) - vec2<i32>(1);
    var most = 0u;
    for (var y = max(tile.y - 1, 0); y <= min(tile.y + 1, last.y); y = y + 1) {
        for (var x = max(tile.x - 1, 0); x <= min(tile.x + 1, last.x); x = x + 1) {
            most = max(most, tiles[u32(y) * look.size.z + u32(x)]);
        }
    }
    var sum = vec4<f32>(0.0);
    if (most > 0u) {
        let reach = i32(most) - 1;
        let edge = vec2<i32>(look.size.xy) - vec2<i32>(1);
        for (var y = max(at.y - reach, 0); y <= min(at.y + reach, edge.y); y = y + 1) {
            for (var x = max(at.x - reach, 0); x <= min(at.x + reach, edge.x); x = x + 1) {
                let cell = (u32(y) * look.size.x + u32(x)) * 4u;
                let count = cells[cell];
                if (count > 0u) {
                    let kernel = look.kernels[level_at(count, u32(x), u32(y))];
                    let d = abs(vec2<i32>(x, y) - at);
                    if (max(d.x, d.y) <= i32(kernel.x)) {
                        let weight = exp(-f32(d.x * d.x + d.y * d.y) * kernel.y) * kernel.z;
                        let hits = f32(count);
                        let colors = vec3<f32>(f32(cells[cell + 1u]), f32(cells[cell + 2u]), f32(cells[cell + 3u]));
                        sum = sum + vec4<f32>(colors * (hits / f32(min(count, FULL))), hits) * weight;
                    }
                }
            }
        }
    }
    var out = vec4<f32>(0.0);
    if (sum.w > 0.0) {
        let density = sum.w * look.scale.x;
        var level = log(1.0 + density);
        if (density < 0.001) {
            level = density - 0.5 * density * density;
        }
        out = vec4<f32>(min(sum.xyz / (sum.w * 255.0), vec3<f32>(1.0)), level * look.scale.z);
    }
    textureStore(levels, at, out);
}
