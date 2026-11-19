use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_wgpu::wgpu;
use fractique_core::scene::{MAX_SHAPES, MAX_SHEETS, MAX_SWELLS, Shape, Sheets};
use fractique_core::{RenderOptions, Scene, math};

use crate::field::{Batched, strip};
use crate::pace;
use crate::preview::{LEVELS, guarded};

// However wrong the guess of a batch's time, its pixels at their dearest take no longer than this.
const DEAREST: f64 = 0.05;
// Units of work a second of a card nothing is known about yet. No card is taken to be slower: one that is has
// its batch end late, and the picture is the processor's.
const UNHURRIED: f64 = 5.0e7;
// Units of work one submission holds at the very most, whatever the card has shown: its pixels times the units
// a pixel takes at most. It is checked before anything is sent.
const MOST_WORK: f64 = 1.0e8;
// The pixels of the smallest batch: one workgroup.
const LEAST: u64 = 64;
// The units of sheets.wgsl: a shape whose distance is worked out, and a swell of a wave.
const WHOLE: f64 = 4.0;
const SWELL: f64 = 3.0;
const ROWS: usize = 7;
// Pixels the largest number a shape's distance is made from may measure: above it 32-bit floats are off by more
// than a sixteenth of a pixel, and the processor draws the scene.
const FINEST: f64 = (1u32 << 20) as f64;
// fractique-core's sheets.rs: how far the distance is taken again for its rise.
const STEP: f64 = 1e-3;
// Pixels of a diagonal in one of its numbers, two bits each, so that a 32-bit float holds them exactly.
const HELD: usize = 12;

// The scene is a wallpaper: sheets and nothing over them.
pub(crate) fn drawn(scene: &Scene) -> bool {
    scene.sheets.is_some() && scene.layers.is_empty()
}

// What the card's picture of a wallpaper depends on besides its size.
pub(crate) fn key(scene: &Scene) -> String {
    serde_json::to_string(&(&scene.sheets, scene.background)).unwrap_or_default()
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    size: [u32; 4],
    view: [f32; 4],
    light: [f32; 4],
    look: [f32; 4],
    shade: [f32; 4],
    ground: [f32; 4],
    plies: [[[f32; 4]; 3]; MAX_SHEETS],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Batch {
    region: [u32; 4],
    stride: [u32; 4],
}

fn turn(degrees: f64) -> (f64, f64) {
    let angle = degrees.to_radians();
    (math::cos(angle), math::sin(angle))
}

// fractique-core's sheets.rs, Solid::new: the rows of a shape, the units its distance costs, and the largest
// number that distance is made from at the far edge of the picture.
fn solid(shape: &Shape, edge: f64) -> ([[f64; 4]; ROWS], f64, f64) {
    let mut rows = [[0.0; 4]; ROWS];
    let from = |center: [f64; 2]| (center[0] * center[0] + center[1] * center[1]).sqrt() + edge;
    match shape {
        Shape::Box {
            center,
            half,
            corners,
            rotation,
        } => {
            let (c, s) = turn(*rotation);
            let reach = (half[0] * half[0] + half[1] * half[1]).sqrt();
            rows[0] = [center[0], center[1], c, s];
            rows[1] = [reach, half[0], half[1], 0.0];
            rows[2] = [half[0], half[1], 0.0, 0.0];
            rows[3] = *corners;
            (rows, WHOLE, from(*center) + reach)
        }
        Shape::Petal {
            base,
            length,
            width,
            rotation,
        } => {
            let (a, b) = (width / 2.0, length / 2.0);
            let radius = (a + b * b / a) / 2.0;
            let (c, s) = turn(*rotation);
            let center = [base[0] + s * b, base[1] - c * b];
            rows[0] = [center[0], center[1], c, s];
            rows[1] = [b, a, b, 1.0];
            rows[2] = [radius, radius - a, b, 0.0];
            (rows, WHOLE, from(center) + radius)
        }
        Shape::Ring {
            center,
            radius,
            thickness,
            span,
            rotation,
        } => {
            let outer = radius + thickness / 2.0;
            let (c, s) = turn(*rotation);
            let (along, aside) = turn(span / 2.0);
            let open = if *span < 360.0 { 1.0 } else { 0.0 };
            rows[0] = [center[0], center[1], c, s];
            rows[1] = [outer, outer, outer, 2.0];
            rows[2] = [*radius, thickness / 2.0, open, 0.0];
            rows[3] = [aside, along, 0.0, 0.0];
            (rows, WHOLE, from(*center) + outer)
        }
        Shape::Wave {
            level,
            swells,
            thickness,
            rotation,
        } => {
            let (c, s) = turn(*rotation);
            let far = level.abs() + edge;
            let mut size = far;
            rows[0] = [0.0, *level, c, s];
            rows[1] = [0.0, 0.0, 0.0, 3.0];
            rows[2] = [thickness / 2.0, swells.len() as f64, 0.0, 0.0];
            for (row, swell) in rows[3..].iter_mut().zip(swells) {
                let rate = math::TAU / swell.length;
                *row = [swell.height, rate, swell.shift, 0.0];
                size += swell.height.abs() * (rate * far + swell.shift.abs()).max(1.0);
            }
            (rows, WHOLE + SWELL * swells.len().min(MAX_SWELLS) as f64, size)
        }
    }
}

// fractique-core's sheets.rs, Solid::distance of a box that is not turned, from its center.
fn within(half: [f64; 2], corners: &[f64; 4], px: f64, py: f64) -> f64 {
    let [hx, hy] = half;
    let (qx, qy) = (px.abs() - hx, py.abs() - hy);
    let signs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for (radius, (sx, sy)) in corners.iter().zip(signs) {
        if *radius <= 0.0 {
            continue;
        }
        let (vx, vy) = ((px - sx * (hx - radius)) * sx, (py - sy * (hy - radius)) * sy);
        if vx > 0.0 && vy > 0.0 {
            let round = (vx * vx + vy * vy).sqrt() - radius;
            return if qx <= 0.0 && qy <= 0.0 {
                round.max(qx.max(qy))
            } else {
                round
            };
        }
    }
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0)
}

struct View {
    scale: f64,
    half: (f64, f64),
    size: [u32; 2],
}

// Where the centers of pixels lie on the diagonal of the corner of a box that is not turned, the one both
// steps of the rise lead away from, the last bit of the processor's 64-bit numbers decides which side is
// nearer, or that neither is. The pixels across of that diagonal less the pixels down, its first row, and for
// every row from it what the processor finds: 0 to leave it to the card, 1 no rise, 2 a rise across only,
// 3 a rise down only.
fn diagonal(center: [f64; 2], half: [f64; 2], corners: &[f64; 4], view: &View) -> Option<(f64, u32, Vec<u8>)> {
    let View {
        scale,
        half: edge,
        size,
    } = *view;
    let lean = (center[0] - center[1] + half[1] - half[0] + edge.0 - edge.1) / scale;
    let ahead = lean.round();
    if (lean - ahead).abs() > 1.0 / 16.0 {
        return None;
    }
    let row = |y: f64| ((y + edge.1) / scale).clamp(0.0, f64::from(size[1]));
    let (from, to) = (row(center[1] - half[1]).floor() as u32, row(center[1]).ceil() as u32);
    let code = |y: u32| {
        let x = f64::from(y) + ahead;
        if x < 0.0 || x >= f64::from(size[0]) {
            return 0;
        }
        let (px, py) = ((x + 0.5) * scale - edge.0, (f64::from(y) + 0.5) * scale - edge.1);
        let (lx, ly) = (px - center[0], py - center[1]);
        if lx >= 0.0 || ly >= 0.0 || lx.abs() > half[0] || ly.abs() > half[1] {
            return 0;
        }
        let d = within(half, corners, lx, ly);
        let gx = within(half, corners, (px + STEP) - center[0], ly) - d;
        let gy = within(half, corners, lx, (py + STEP) - center[1]) - d;
        match (gx, gy) {
            (0.0, 0.0) => 1,
            (gx, 0.0) if gx < 0.0 => 2,
            (0.0, gy) if gy < 0.0 => 3,
            _ => 0,
        }
    };
    let codes: Vec<u8> = (from..to).map(code).collect();
    let first = codes.iter().position(|code| *code != 0)?;
    let last = codes.iter().rposition(|code| *code != 0)?;
    Some((ahead, from + first as u32, codes[first..=last].to_vec()))
}

struct Plan {
    frame: Box<Frame>,
    // The shapes, seven rows each, then the diagonals of the boxes that have one.
    shapes: Vec<[f32; 4]>,
    // Units of work a pixel takes at most.
    cost: f64,
}

// Nothing when 32-bit floats would show in the picture.
fn planned(scene: &Scene, stack: &Sheets, options: &RenderOptions) -> Option<Plan> {
    let (wide, high) = (f64::from(options.width), f64::from(options.height));
    let scale = 2.0 / wide.min(high);
    let half = (wide * scale / 2.0, high * scale / 2.0);
    let angle = stack.light.to_radians();
    let far = stack.shadow_blur + stack.shadow_offset + 2.0 * scale;
    let edge = (half.0 * half.0 + half.1 * half.1).sqrt() + far;
    let mut frame = Box::new(Frame {
        size: [options.width, options.height, stack.stack.len() as u32, 0],
        view: [scale, wide / 2.0, high / 2.0, half.0.max(half.1)].map(|value| value as f32),
        light: [math::cos(angle), -math::sin(angle), far, 0.0].map(|value| value as f32),
        look: [stack.rim, stack.rim_width, stack.bevel, stack.gradient].map(|value| value as f32),
        shade: [stack.shadow, stack.shadow_offset, stack.shadow_blur, 0.0].map(|value| value as f32),
        ground: [scene.background[0], scene.background[1], scene.background[2], 1.0].map(|value| value as f32),
        plies: [[[0.0; 4]; 3]; MAX_SHEETS],
    });
    let view = View {
        scale,
        half,
        size: [options.width, options.height],
    };
    let rows = ROWS
        * stack
            .stack
            .iter()
            .map(|sheet| sheet.shapes.len().min(MAX_SHAPES))
            .sum::<usize>();
    frame.size[3] = rows as u32;
    let mut shapes = Vec::with_capacity(rows);
    let mut diagonals: Vec<[f32; 4]> = Vec::new();
    let mut cost = 0.0;
    for (ply, sheet) in frame.plies.iter_mut().zip(&stack.stack) {
        let (first, to) = (shapes.len() / ROWS, sheet.color_to.unwrap_or([0.0; 3]));
        let flag = if sheet.color_to.is_some() { 1.0 } else { 0.0 };
        *ply = [
            [sheet.color[0], sheet.color[1], sheet.color[2], sheet.opacity],
            [to[0], to[1], to[2], flag],
            [first as f64, sheet.shapes.len() as f64, 0.0, 0.0],
        ]
        .map(|row| row.map(|value| value as f32));
        let (mut all, mut dearest) = (0.0, 0.0f64);
        for shape in sheet.shapes.iter().take(MAX_SHAPES) {
            let (rows, units, size) = solid(shape, edge);
            let fine = size / scale;
            if fine.is_nan() || fine > FINEST {
                return None;
            }
            let mut rows = rows.map(|row| row.map(|value| value as f32));
            if let Shape::Box {
                center,
                half,
                corners,
                rotation,
            } = shape
                && turn(*rotation) == (1.0, 0.0)
                && let Some((ahead, first, codes)) = diagonal(*center, *half, corners, &view)
            {
                rows[4] = [ahead as f32, first as f32, codes.len() as f32, diagonals.len() as f32];
                let numbers: Vec<f32> = codes
                    .chunks(HELD)
                    .map(|held| held.iter().rev().fold(0u32, |all, code| all << 2 | u32::from(*code)) as f32)
                    .collect();
                diagonals.extend(numbers.chunks(4).map(|four| {
                    let mut row = [0.0; 4];
                    row[..four.len()].copy_from_slice(four);
                    row
                }));
            }
            shapes.extend(rows);
            all += units;
            dearest = dearest.max(units);
        }
        cost += 2.0 * (all + dearest);
    }
    shapes.extend(diagonals);
    Some(Plan { frame, shapes, cost })
}

// How fast the card has drawn wallpapers: what the next batch is sized by.
#[derive(Clone, Copy, Default)]
struct Rate {
    pixels: f64,
    units: f64,
}

fn bound(rate: &Rate) -> f64 {
    (rate.units.max(UNHURRIED) * DEAREST).min(MOST_WORK)
}

struct Page {
    size: [u32; 2],
    levels: wgpu::TextureView,
}

// What the wallpapers share on the graphics card: their shader, compiled when one is first asked for, how fast
// the card has been at them, and the texture of the picture before this one.
pub(crate) struct Stacks {
    device: wgpu::Device,
    queue: wgpu::Queue,
    failed: Arc<AtomicBool>,
    able: bool,
    layout: Arc<wgpu::BindGroupLayout>,
    pipe: Arc<OnceLock<wgpu::ComputePipeline>>,
    asked: AtomicBool,
    rate: Arc<Mutex<Rate>>,
    spare: Arc<Mutex<Vec<Page>>>,
}

#[derive(Clone, Copy)]
enum Stage {
    Start,
    First([u32; 2]),
    Done,
}

pub(crate) struct Layer {
    queue: wgpu::Queue,
    pipe: wgpu::ComputePipeline,
    rate: Arc<Mutex<Rate>>,
    spare: Arc<Mutex<Vec<Page>>>,
    size: [u32; 2],
    cost: f64,
    page: Option<Page>,
    batch: wgpu::Buffer,
    counter: wgpu::Buffer,
    group: wgpu::BindGroup,
    stage: Stage,
    flying: Option<u64>,
    drew: bool,
    last: Option<(u64, f64)>,
}

fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty,
        count: None,
    };
    let buffer = |ty| wgpu::BindingType::Buffer {
        ty,
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let uniform = buffer(wgpu::BufferBindingType::Uniform);
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fractique sheets"),
        entries: &[
            entry(0, uniform),
            entry(1, uniform),
            entry(2, buffer(wgpu::BufferBindingType::Storage { read_only: false })),
            entry(
                3,
                wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: LEVELS,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
            ),
            entry(4, buffer(wgpu::BufferBindingType::Storage { read_only: true })),
        ],
    })
}

fn pipeline(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fractique sheets"),
        source: wgpu::ShaderSource::Wgsl(include_str!("sheets.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fractique sheets"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("fractique sheets"),
        layout: Some(&layout),
        module: &module,
        entry_point: Some("first"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

impl Stacks {
    pub(crate) fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        failed: Arc<AtomicBool>,
    ) -> Self {
        let stored = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING;
        let able = adapter
            .get_texture_format_features(LEVELS)
            .allowed_usages
            .contains(stored);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            failed,
            able,
            layout: Arc::new(layout(device)),
            pipe: Arc::default(),
            asked: AtomicBool::new(false),
            rate: Arc::default(),
            spare: Arc::default(),
        }
    }

    // The shader is being compiled: a wallpaper cannot be drawn yet, and soon can.
    pub(crate) fn pending(&self) -> bool {
        self.asked.load(Ordering::Relaxed) && self.pipe.get().is_none()
    }

    fn compile(&self, ctx: &egui::Context) {
        if self.asked.swap(true, Ordering::Relaxed) {
            return;
        }
        let (device, failed, ctx) = (self.device.clone(), self.failed.clone(), ctx.clone());
        let (layout, pipe) = (self.layout.clone(), self.pipe.clone());
        let build = move || {
            let made = guarded(&device, &failed, &ctx, || pipeline(&device, &layout));
            if !failed.load(Ordering::Relaxed) {
                let _ = pipe.set(made);
            }
            ctx.request_repaint();
        };
        // A driver takes up to seconds to compile a shader: the window does not wait for it.
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(build);
        #[cfg(target_arch = "wasm32")]
        build();
    }

    fn page(&self, size: [u32; 2]) -> Page {
        let mut spare = self.spare.lock().unwrap_or_else(|e| e.into_inner());
        spare.retain(|page| page.size == size);
        if let Some(page) = spare.pop() {
            return page;
        }
        let levels = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("fractique levels"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: LEVELS,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        Page { size, levels }
    }

    // The wallpaper made ready for the card, if the card draws it at this size: nothing while the shader is
    // being compiled, and nothing for a scene whose smallest batch could take more than the bound.
    pub(crate) fn layer(&self, scene: &Scene, options: &RenderOptions, ctx: &egui::Context) -> Option<Box<Layer>> {
        let stack = scene
            .sheets
            .as_ref()
            .filter(|_| self.able && options.supersample == 1)?;
        let plan = planned(scene, stack, options)?;
        let rate = *self.rate.lock().unwrap_or_else(|e| e.into_inner());
        if plan.shapes.is_empty() || bound(&rate) < LEAST as f64 * plan.cost {
            return None;
        }
        let Some(pipe) = self.pipe.get() else {
            self.compile(ctx);
            return None;
        };
        let size = [options.width, options.height];
        let page = self.page(size);
        let buffer = |size: u64, usage| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractique sheets"),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let frame = buffer(size_of::<Frame>() as u64, wgpu::BufferUsages::UNIFORM);
        self.queue.write_buffer(&frame, 0, bytemuck::bytes_of(&*plan.frame));
        let shapes = buffer(16 * plan.shapes.len() as u64, wgpu::BufferUsages::STORAGE);
        self.queue.write_buffer(&shapes, 0, bytemuck::cast_slice(&plan.shapes));
        let batch = buffer(size_of::<Batch>() as u64, wgpu::BufferUsages::UNIFORM);
        let counter = buffer(16, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        fn whole(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
            wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }
        }
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fractique sheets"),
            layout: &self.layout,
            entries: &[
                whole(0, &frame),
                whole(1, &batch),
                whole(2, &counter),
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&page.levels),
                },
                whole(4, &shapes),
            ],
        });
        Some(Box::new(Layer {
            queue: self.queue.clone(),
            pipe: pipe.clone(),
            rate: self.rate.clone(),
            spare: self.spare.clone(),
            size,
            cost: plan.cost,
            page: Some(page),
            batch,
            counter,
            group,
            stage: Stage::Start,
            flying: None,
            drew: false,
            last: None,
        }))
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        let mut spare = self.spare.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(page) = self.page.take()
            && spare.len() < 2
        {
            spare.push(page);
        }
    }
}

impl Layer {
    pub(crate) fn levels(&self) -> &wgpu::TextureView {
        &self
            .page
            .as_ref()
            .expect("a layer keeps its texture while it lives")
            .levels
    }

    // The pixels of the next batch: as many as would have taken the pace in the batch before, between a quarter
    // of it and four times it, and never more than the bound allows if every one of them took all its units.
    fn budget(&self) -> u64 {
        let rate = *self.rate.lock().unwrap_or_else(|e| e.into_inner());
        let most = bound(&rate) / self.cost;
        let pixels = match self.last {
            Some((pixels, seconds)) => {
                let before = pixels as f64;
                pace::next(before, seconds as f32, pace::slice())
            }
            None if rate.pixels > 0.0 => rate.pixels * f64::from(pace::slice()),
            None => most,
        };
        (pixels.min(most) as u64).max(LEAST)
    }

    fn dispatch(&self, encoder: &mut wgpu::CommandEncoder, region: [u32; 4], stride: u32) {
        let batch = Batch {
            region,
            stride: [stride, 0, 0, 0],
        };
        self.queue.write_buffer(&self.batch, 0, bytemuck::bytes_of(&batch));
        self.queue.write_buffer(&self.counter, 0, &[0; 16]);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fractique sheets"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipe);
        pass.set_bind_group(0, &self.group, &[]);
        let block = 8 * stride;
        pass.dispatch_workgroups(region[2].div_ceil(block), region[3].div_ceil(block), 1);
    }
}

impl Batched for Layer {
    // Sends the next batch: one sample a block of pixels first, when one a pixel would take too long, then one
    // a pixel in bands across the picture.
    fn send(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let [width, height] = self.size;
        if matches!(self.stage, Stage::Start) {
            self.stage = Stage::First([0, 0]);
            let budget = self.budget();
            let blocks = |stride: u32| u64::from(width.div_ceil(stride)) * u64::from(height.div_ceil(stride));
            if budget < blocks(1)
                && let Some(stride) = [2, 4, 8, 16, 32, 64]
                    .into_iter()
                    .find(|stride| blocks(*stride) <= budget)
            {
                self.dispatch(encoder, [0, 0, width, height], stride);
                self.flying = Some(blocks(stride));
                return;
            }
        }
        if let Stage::First(mut at) = self.stage {
            let region = strip(&mut at, self.budget(), self.size);
            self.stage = if at[1] < height { Stage::First(at) } else { Stage::Done };
            self.dispatch(encoder, region, 1);
            self.flying = Some(u64::from(region[2]) * u64::from(region[3]));
        }
    }

    // Takes in how long the batch took and a sixteenth of the units its pixels took.
    fn took(&mut self, counted: u32, seconds: f32) {
        let Some(pixels) = self.flying.take() else {
            return;
        };
        let seconds = f64::from(seconds).max(1e-4);
        let mut rate = self.rate.lock().unwrap_or_else(|e| e.into_inner());
        rate.pixels = if rate.pixels > 0.0 {
            0.5 * (rate.pixels + pixels as f64 / seconds)
        } else {
            pixels as f64 / seconds
        };
        rate.units = rate.units.max(f64::from(counted) * 16.0 / seconds);
        self.drew = true;
        self.last = Some((pixels, seconds));
    }

    fn counter(&self) -> &wgpu::Buffer {
        &self.counter
    }

    fn drew(&self) -> bool {
        self.drew
    }

    fn done(&self) -> bool {
        matches!(self.stage, Stage::Done)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fractique_core::Control;
    use fractique_core::scene::{Sheet, Swell};
    use fractique_core::wallpaper::{Layout, Material, Request, random_wallpaper};
    use web_time::Instant;

    use super::*;
    use crate::chaos::tests::{apart, card, save};

    fn options(width: u32, height: u32) -> RenderOptions {
        RenderOptions {
            width,
            height,
            samples_per_pixel: 32,
            supersample: 1,
        }
    }

    fn wallpaper(seed: u64, layout: Layout, material: Material, dark: bool, options: &RenderOptions) -> Scene {
        let request = Request {
            layout: Some(layout),
            material: Some(material),
            ratio: f64::from(options.height) / f64::from(options.width),
            dark,
            hue: None,
        };
        random_wallpaper(seed, &request)
    }

    fn sizes(name: &str, usual: &str) -> Vec<[u32; 2]> {
        std::env::var(name)
            .unwrap_or_else(|_| usual.to_owned())
            .split(',')
            .map(|size| {
                let sides: Vec<u32> = size.split('x').filter_map(|side| side.parse().ok()).collect();
                [sides[0], sides[1]]
            })
            .collect()
    }

    fn seeds() -> u64 {
        std::env::var("FRACTIQUE_COMPARE_SEEDS")
            .ok()
            .and_then(|seeds| seeds.parse().ok())
            .unwrap_or(3)
    }

    #[test]
    fn the_shader_is_valid() {
        let module = wgpu::naga::front::wgsl::parse_str(include_str!("sheets.wgsl")).expect("the shader parses");
        let flags = wgpu::naga::valid::ValidationFlags::all();
        wgpu::naga::valid::Validator::new(flags, wgpu::naga::valid::Capabilities::empty())
            .validate(&module)
            .expect("the shader is valid");
    }

    #[test]
    fn every_generated_wallpaper_is_one_the_card_takes() {
        for [width, height] in [[1920, 1080], [1080, 2400], [7680, 4320]] {
            let options = options(width, height);
            for (_, layout) in Layout::ALL {
                for (_, material) in Material::ALL {
                    for seed in 1..=4 {
                        let scene = wallpaper(seed, layout, material, seed % 2 == 0, &options);
                        let stack = scene.sheets.as_ref().expect("a wallpaper has sheets");
                        let plan = planned(&scene, stack, &options).expect("32-bit floats hold it");
                        assert!(plan.shapes.len() >= plan.frame.size[3] as usize);
                        assert_eq!(plan.frame.size[3] as usize % ROWS, 0);
                        assert!(LEAST as f64 * plan.cost <= bound(&Rate::default()), "{layout:?} {seed}");
                    }
                }
            }
        }
    }

    fn heaviest() -> Scene {
        let swell = Swell {
            height: 0.1,
            length: 1.0,
            shift: 0.0,
        };
        let wave = Shape::Wave {
            level: 0.0,
            swells: vec![swell; MAX_SWELLS],
            thickness: 0.1,
            rotation: 0.0,
        };
        let sheet = Sheet {
            color: [0.5; 3],
            color_to: None,
            opacity: 1.0,
            shapes: vec![wave; MAX_SHAPES],
        };
        let mut scene = wallpaper(1, Layout::Bands, Material::Paper, false, &options(640, 360));
        scene.sheets.as_mut().expect("a wallpaper has sheets").stack = vec![sheet; MAX_SHEETS];
        scene.validate().expect("the scene is within the format's limits");
        scene
    }

    #[test]
    fn the_work_of_a_batch_is_bounded_before_it_is_sent() {
        let (scene, options) = (heaviest(), options(640, 360));
        let plan = planned(&scene, scene.sheets.as_ref().expect("sheets"), &options).expect("it is planned");
        let each = WHOLE + SWELL * MAX_SWELLS as f64;
        assert_eq!(plan.cost, (MAX_SHEETS * 2 * (MAX_SHAPES + 1)) as f64 * each);
        let quick = Rate {
            units: 1.0e12,
            ..Rate::default()
        };
        assert_eq!(bound(&quick), MOST_WORK);
        assert!(LEAST as f64 * plan.cost <= bound(&quick));
        assert!(LEAST as f64 * plan.cost > bound(&Rate::default()));
    }

    #[test]
    fn a_shape_too_large_for_single_precision_is_the_processors() {
        let options = options(1920, 1080);
        let mut scene = wallpaper(1, Layout::Discs, Material::Paper, false, &options);
        let stack = scene.sheets.as_mut().expect("a wallpaper has sheets");
        stack.stack[0].shapes.push(Shape::Ring {
            center: [0.0, 3000.0],
            radius: 3000.0,
            thickness: 0.1,
            span: 360.0,
            rotation: 0.0,
        });
        scene.validate().expect("the scene is valid");
        assert!(planned(&scene, scene.sheets.as_ref().expect("sheets"), &options).is_none());
    }

    #[test]
    fn the_processors_ties_on_the_diagonal_of_a_corner_are_told_to_the_card() {
        let view = View {
            scale: 2.0 / 1080.0,
            half: (1920.0 / 1080.0, 1.0),
            size: [1920, 1080],
        };
        let cell = 2.0 * view.half.0 / 3.0;
        let (center, half) = ([-view.half.0 + cell * 1.5, 0.0], [cell / 2.0; 2]);
        let (ahead, first, codes) = diagonal(center, half, &[0.0; 4], &view).expect("the diagonal meets pixels");
        assert_eq!((ahead, first, codes.len()), (420.0, 220, 320));
        assert!(codes.iter().all(|code| (1..=3).contains(code)));
        assert!(codes.contains(&1));
        assert!(diagonal(center, half, &[cell / 2.0; 4], &view).is_none());
        let moved = [center[0] + view.scale / 3.0, center[1]];
        assert!(diagonal(moved, half, &[0.0; 4], &view).is_none());
    }

    // A software adapter compiles the shader when it is first run, and is late with that picture.
    fn warmed() -> Option<crate::chaos::tests::Card> {
        let mut card = card()?;
        let options = options(64, 64);
        let _ = card.drawn(&wallpaper(99, Layout::Tiles, Material::Glass, true, &options), &options);
        Some(card)
    }

    // The bytes of the two pictures that differ, out of a hundred, and the largest difference of one.
    fn unlike(one: &[u8], other: &[u8]) -> (f64, u8) {
        let differing = one.iter().zip(other).filter(|(a, b)| a != b).count();
        let largest = one.iter().zip(other).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
        (100.0 * differing as f64 / one.len().max(1) as f64, largest)
    }

    #[test]
    fn the_graphics_card_draws_the_wallpaper_the_processor_renders() {
        let Some(mut card) = warmed() else {
            eprintln!("no graphics card with compute shaders: nothing is compared");
            return;
        };
        for [width, height] in [[320, 200], [180, 400]] {
            let options = options(width, height);
            for (seed, (_, layout)) in Layout::ALL.into_iter().enumerate() {
                let material = Material::ALL[seed % Material::ALL.len()].1;
                let scene = wallpaper(seed as u64, layout, material, seed % 2 == 0, &options);
                let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
                let (quick, _, _) = card.drawn(&scene, &options).expect("the graphics card draws it");
                let (mean, largest) = apart(&quick, &exact.rgba, width as usize, height as usize);
                assert!(
                    mean < 0.1 && largest < 2.0,
                    "{layout:?}: {mean:.3} on average, {largest:.2} at most"
                );
            }
        }
    }

    #[test]
    fn the_heaviest_scene_is_drawn_in_batches_within_the_bound() {
        let Some(mut card) = warmed() else {
            eprintln!("no graphics card with compute shaders: nothing is drawn");
            return;
        };
        let larger = options(640, 360);
        let _ = card.drawn(&wallpaper(1, Layout::Waves, Material::Paper, false, &larger), &larger);
        let (scene, options) = (heaviest(), options(96, 64));
        let Some((quick, _, _)) = card.drawn(&scene, &options) else {
            eprintln!("the card is too slow for the heaviest scene: the processor renders it");
            return;
        };
        let (slowest, batches) = card.chaos.slowest().expect("the picture was drawn in batches");
        eprintln!("the heaviest scene: {batches} batches, the longest {slowest:.3} s");
        let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
        let (mean, largest) = apart(&quick, &exact.rgba, 96, 64);
        assert!(
            mean < 0.1 && largest < 2.0,
            "{mean:.3} on average, {largest:.2} at most"
        );
    }

    // FRACTIQUE_COMPARE=folder cargo test --release -p fractique-app --lib every_wallpaper -- --ignored --nocapture
    // writes the table of differences, and for every scene the picture of the graphics card above the processor's.
    // FRACTIQUE_COMPARE_SIZES=1920x1080,1080x2400, FRACTIQUE_COMPARE_SEEDS=3 and FRACTIQUE_COMPARE_ONLY=tiles.
    #[test]
    #[ignore = "a report, not a check"]
    fn every_wallpaper_on_the_graphics_card_beside_the_processor() {
        let mut card = warmed().expect("a graphics card with compute shaders");
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        println!("| Layout | Material | Size | Pictures | Mean | Largest | Bytes that differ, % | Largest of a byte |");
        println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
        let mut all = (0.0, 0f64, 0.0, 0u8, 0u32);
        for [width, height] in sizes("FRACTIQUE_COMPARE_SIZES", "1920x1080,1080x2400") {
            let options = options(width, height);
            for (layout_name, layout) in Layout::ALL {
                for (material_name, material) in Material::ALL {
                    let name = format!("{layout_name}-{material_name}");
                    if only.as_ref().is_some_and(|only| !name.contains(only.as_str())) {
                        continue;
                    }
                    let mut row = (0.0, 0f64, 0.0, 0u8, 0u32);
                    for seed in 1..=seeds() {
                        for dark in [false, true] {
                            let scene = wallpaper(seed, layout, material, dark, &options);
                            let exact =
                                fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
                            let Some((quick, _, _)) = card.drawn(&scene, &options) else {
                                println!("| {layout_name} | {material_name} | {width} × {height} | not drawn |");
                                continue;
                            };
                            let (mean, largest) = apart(&quick, &exact.rgba, width as usize, height as usize);
                            let (share, byte) = unlike(&quick, &exact.rgba);
                            row = (
                                row.0 + mean,
                                row.1.max(largest),
                                row.2 + share,
                                row.3.max(byte),
                                row.4 + 1,
                            );
                            if let Some(folder) = &folder {
                                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                                let tone = if dark { "dark" } else { "light" };
                                let file = format!("{name}-{seed}-{tone}-{width}x{height}.png");
                                save(&folder.join(file), &both, width, height * 2);
                            }
                        }
                    }
                    let count = f64::from(row.4.max(1));
                    println!(
                        "| {layout_name} | {material_name} | {width} × {height} | {} | {:.3} | {:.2} | {:.2} | {} |",
                        row.4,
                        row.0 / count,
                        row.1,
                        row.2 / count,
                        row.3
                    );
                    all = (
                        all.0 + row.0,
                        all.1.max(row.1),
                        all.2 + row.2,
                        all.3.max(row.3),
                        all.4 + row.4,
                    );
                }
            }
        }
        let count = f64::from(all.4.max(1));
        println!(
            "| all | all | all | {} | {:.3} | {:.2} | {:.2} | {} |",
            all.4,
            all.0 / count,
            all.1,
            all.2 / count,
            all.3
        );
    }

    // The seconds to the first and to the whole picture of a wallpaper of every layout, at the sizes given in
    // FRACTIQUE_SPEED_SIZES (1920x1080,3840x2160), beside the processor's render.
    #[test]
    #[ignore = "a report, not a check"]
    fn how_fast_the_graphics_card_draws_wallpapers() {
        let mut card = warmed().expect("a graphics card with compute shaders");
        println!(
            "| Layout | Size | Units a pixel | Card: first, s | Card: whole, s | Batches | Longest, s | Processor, s |"
        );
        println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
        for [width, height] in sizes("FRACTIQUE_SPEED_SIZES", "1920x1080,3840x2160") {
            let options = options(width, height);
            for (index, (name, layout)) in Layout::ALL.into_iter().enumerate() {
                let material = Material::ALL[index % Material::ALL.len()].1;
                let scene = wallpaper(7, layout, material, index % 2 == 0, &options);
                let cost =
                    planned(&scene, scene.sheets.as_ref().expect("sheets"), &options).map_or(0.0, |plan| plan.cost);
                let started = Instant::now();
                fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
                let exact = started.elapsed().as_secs_f32();
                // The second run is the one that counts: the first compiles the shader and learns the card's pace.
                let _ = card.drawn(&scene, &options);
                match card.drawn(&scene, &options) {
                    Some((_, first, full)) => {
                        let (slowest, batches) = card.chaos.slowest().unwrap_or_default();
                        println!(
                            "| {name} | {width} × {height} | {cost:.0} | {first:.3} | {full:.3} | {batches} | {slowest:.3} | {exact:.3} |"
                        );
                    }
                    None => println!("| {name} | {width} × {height} | {cost:.0} | not drawn | | | | {exact:.3} |"),
                }
            }
        }
    }
}
