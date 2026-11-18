use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_wgpu::wgpu;
use fractique_core::scene::Transform;
use fractique_core::{Flame, Kind, RenderOptions, Scene, Tone, math, palette};
use web_time::Instant;

use crate::orbit::{self, Recipe};
use crate::preview::{LEVELS, guarded};

const TRANSFORMS: usize = 64;
const VARIATIONS: usize = 256;
const GROUP: u32 = orbit::GROUP;
const MOST_WALKERS: u32 = 1 << 18;
const FUSE: u32 = 24;
// The first picture is drawn from this many samples a pixel: the density estimation makes it whole.
pub const FIRST_SAMPLES: u32 = 2;
const TILE: u32 = 16;
const KERNELS: usize = 20;
// fractique-core's tone.rs: the widest kernel, its share of the shorter side.
const MAX_SPREAD: f64 = 9.0;
const SPREAD_SHARE: f64 = 0.004;
const SPREAD_CURVE: f64 = 0.4;
// fractique-core's deep.rs: below this share of the points in view the processor draws the view from particles.
const IN_VIEW: f64 = 0.125;
// Histograms and levels of one run, in bytes.
const ROOM: u64 = 1 << 30;
// Cells a unit, times the distance of the view from the origin: above it 32-bit coordinates show as a grid.
const FINEST: f64 = 4.0e5;
// A batch is sized to take about this long, so that the window keeps its pace.
const PACE: f32 = 0.04;
// However slow the graphics card, a run takes about this many batches at most.
const MOST_BATCHES: u64 = 16;
// A walker of an orbit takes steps that plot nothing before its first point: the walkers of a layer together
// take up to this many times the points asked of them in a batch.
const UNSEEN: u64 = 4;
// Whatever the pace asks for, the walkers of all layers together take no more steps in one submission than this
// many a pixel, and no more than the second number: a shader that works for seconds takes the screen with it. A
// step of a flame counts once for every four variations it may apply.
const MOST_STEPS: u64 = 4;
const MOST_IN_ALL: u64 = 1 << 22;
const HEAVY: u64 = 4;
// The cells the density estimation of one submission may look at, all layers together: for every cell of a layer
// those its widest kernel reaches.
const MOST_TAPS: u64 = 3 << 29;
// An orbit whose walkers would take more steps than this many a pixel in all is the processor's.
const MOST_WORK: u64 = 512;
// A submission that takes longer than this many milliseconds ends the card's part for the session.
const LONGEST: u32 = 500;
// A run whose walkers have plotted nothing after this many batches is given up.
const IDLE_BATCHES: u32 = 64;
const RECIPES: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Xf {
    affine: [f32; 4],
    shift: [f32; 4],
    post: [f32; 4],
    after: [f32; 4],
    range: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Var {
    kind: u32,
    weight: f32,
    spare: [f32; 2],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Packed {
    counts: [u32; 4],
    view: [f32; 4],
    scale: [f32; 4],
    extent: [u32; 4],
    total: [f32; 4],
    xfs: [Xf; TRANSFORMS + 1],
    vars: [Var; VARIATIONS],
    palette: [[u32; 4]; 64],
    turns: [[f32; 4]; 32],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Batch {
    steps: u32,
    fuse: u32,
    seed: [u32; 2],
    layer: u32,
    walkers: u32,
    budget: u32,
    spare: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Look {
    size: [u32; 4],
    scale: [f32; 4],
    kernels: [[f32; 4]; KERNELS],
    widths: [[u32; 4]; 1024],
}

fn xf(transform: &Transform, cumulative: f64, vars: &mut Vec<Var>) -> Option<Xf> {
    let first = vars.len();
    for variation in &transform.variations {
        let kind = Kind::ALL.iter().position(|kind| *kind == variation.kind)?;
        let mut params = [0f32; 4];
        for (index, (_, default)) in variation.kind.params().iter().enumerate() {
            params[index] = variation.params.get(index).copied().unwrap_or(*default) as f32;
        }
        vars.push(Var {
            kind: kind as u32,
            weight: variation.weight as f32,
            spare: [0.0; 2],
            params,
        });
    }
    if vars.len() > VARIATIONS {
        return None;
    }
    let [a, b, c, d, e, f] = transform.affine.map(|value| value as f32);
    let post = transform.post.unwrap_or([0.0; 6]).map(|value| value as f32);
    Some(Xf {
        affine: [a, b, c, d],
        shift: [e, f, transform.color as f32, transform.color_speed as f32],
        post: [post[0], post[1], post[2], post[3]],
        after: [post[4], post[5], transform.opacity as f32, cumulative as f32],
        range: [
            first as u32,
            (vars.len() - first) as u32,
            u32::from(transform.post.is_some()),
            0,
        ],
    })
}

// The flame as the shader reads it and the share of its points that are plotted, or nothing when the flame or
// its view is one the graphics card does not draw.
fn packed(flame: &Flame, [width, height]: [u32; 2], finest: f64) -> Option<(Box<Packed>, f64)> {
    if flame.transforms.is_empty() || flame.transforms.len() > TRANSFORMS {
        return None;
    }
    let camera = &flame.camera;
    let per_unit = camera.zoom * f64::from(width.min(height)) / 4.0;
    let far = camera.center[0].abs().max(camera.center[1].abs()) + 0.5 * f64::from(width.max(height)) / per_unit;
    if per_unit * far > finest {
        return None;
    }
    let mut data: Box<Packed> = Box::new(bytemuck::Zeroable::zeroed());
    let mut vars = Vec::new();
    let (mut total, mut plotted) = (0.0, 0.0);
    for (index, transform) in flame.transforms.iter().enumerate() {
        total += transform.weight;
        plotted += transform.weight * transform.opacity.clamp(0.0, 1.0);
        data.xfs[index] = xf(transform, total, &mut vars)?;
    }
    if let Some(transform) = &flame.final_transform {
        data.xfs[TRANSFORMS] = xf(transform, 0.0, &mut vars)?;
    }
    data.vars[..vars.len()].copy_from_slice(&vars);
    let copies = flame.symmetry.unsigned_abs().max(1);
    for copy in 0..copies {
        let angle = math::TAU * f64::from(copy) / f64::from(copies);
        let at = (copy % 2 * 2) as usize;
        data.turns[(copy / 2) as usize][at] = math::cos(angle) as f32;
        data.turns[(copy / 2) as usize][at + 1] = math::sin(angle) as f32;
    }
    for (shade, [r, g, b]) in palette::lut(&flame.palette).into_iter().enumerate() {
        data.palette[shade / 4][shade % 4] = u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16;
    }
    let angle = camera.rotation.to_radians();
    let up = if camera.flip_y { -per_unit } else { per_unit };
    data.counts = [
        flame.transforms.len() as u32,
        copies,
        u32::from(flame.symmetry < 0),
        u32::from(flame.final_transform.is_some()),
    ];
    data.view = [
        camera.center[0] as f32,
        camera.center[1] as f32,
        math::cos(angle) as f32,
        math::sin(angle) as f32,
    ];
    data.scale = [per_unit as f32, up as f32, width as f32, height as f32];
    data.extent = [width, height, 0, 0];
    data.total = [total as f32, 0.0, 0.0, 0.0];
    Some((data, plotted / total))
}

fn widest(tone: &Tone, [width, height]: [u32; 2]) -> (f64, usize) {
    let spread = (SPREAD_SHARE * tone.smoothing * f64::from(width.min(height))).min(MAX_SPREAD);
    (spread, ((spread * 2.0).round() as usize).min(KERNELS - 1))
}

// The most cells the density estimation looks at for one cell of a picture of this size.
fn taps(tone: &Tone, size: [u32; 2]) -> u64 {
    let reach = widest(tone, size).1.div_ceil(2) as u64;
    (2 * reach + 1) * (2 * reach + 1)
}

// The kernels of fractique-core's density estimation for a picture of this size.
fn look(tone: &Tone, [width, height]: [u32; 2]) -> Box<Look> {
    let (spread, top) = widest(tone, [width, height]);
    let mut kernels = [[0f32; 4]; KERNELS];
    for (level, kernel) in kernels.iter_mut().enumerate().take(top + 1) {
        let wide = level as f64 * 0.5;
        let reach = wide.ceil() as i32;
        let sigma = (wide * 0.5).max(0.3);
        let total: f32 = (-reach..=reach)
            .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| math::exp(-f64::from(dx * dx + dy * dy) / (2.0 * sigma * sigma)) as f32)
            .sum();
        *kernel = [reach as f32, (1.0 / (2.0 * sigma * sigma)) as f32, 1.0 / total, 0.0];
    }
    let mut widths = [[0u32; 4]; 1024];
    for count in 1..4096usize {
        let wide = spread / math::pow(count as f64, SPREAD_CURVE);
        if wide >= 0.4 {
            widths[count / 4][count % 4] = ((wide * 2.0).round() as usize).min(top) as u32;
        }
    }
    Box::new(Look {
        size: [width, height, width.div_ceil(TILE), height.div_ceil(TILE)],
        scale: [0.0; 4],
        kernels,
        widths,
    })
}

// What a step of the flame counts for among the steps a submission may take.
fn weight(flame: &Flame) -> u64 {
    let applied = |transform: &Transform| transform.variations.len() as u64;
    let heaviest = flame.transforms.iter().map(applied).max().unwrap_or(0);
    (heaviest + flame.final_transform.as_ref().map_or(0, applied))
        .div_ceil(HEAVY)
        .max(1)
}

fn bound(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

struct Pipelines {
    walk: wgpu::ComputePipeline,
    orbit: wgpu::ComputePipeline,
    survey: wgpu::ComputePipeline,
    resolve: wgpu::ComputePipeline,
    walking: wgpu::BindGroupLayout,
    looking: wgpu::BindGroupLayout,
}

impl Pipelines {
    fn new(device: &wgpu::Device) -> Self {
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
        let written = buffer(wgpu::BufferBindingType::Storage { read_only: false });
        let read = buffer(wgpu::BufferBindingType::Storage { read_only: true });
        let walking = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fractique chaos"),
            entries: &[
                entry(0, uniform),
                entry(1, uniform),
                entry(2, written),
                entry(3, written),
                entry(4, written),
            ],
        });
        let looking = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fractique levels"),
            entries: &[
                entry(0, uniform),
                entry(1, read),
                entry(2, written),
                entry(
                    3,
                    wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: LEVELS,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                ),
            ],
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, source: &str, entry_point: &str| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(entry_point),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry_point),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            walk: pipeline(&walking, include_str!("chaos.wgsl"), "walk"),
            orbit: pipeline(&walking, orbit::SHADER, "walk"),
            survey: pipeline(&looking, include_str!("levels.wgsl"), "survey"),
            resolve: pipeline(&looking, include_str!("levels.wgsl"), "resolve"),
            walking,
            looking,
        }
    }
}

// What a layer needs while its samples are plotted: with the last of them its histogram and walkers are freed.
struct Work {
    batch: wgpu::Buffer,
    look: wgpu::Buffer,
    landed: wgpu::Buffer,
    walking: wgpu::BindGroup,
    looking: wgpu::BindGroup,
}

struct Layer {
    work: Option<Work>,
    levels: wgpu::TextureView,
    kernels: Box<Look>,
    plotted: f64,
    zoom: f64,
    weight: u64,
    // A flame's walkers plot the points they are asked for. An orbit's tell how many they plotted, and its levels
    // are made from the points that are told of: with the next batch, and once more after the last.
    orbit: bool,
    done: u64,
    resolved: u64,
}

impl Layer {
    fn pictured(&self) -> bool {
        if self.orbit { self.resolved > 0 } else { self.done > 0 }
    }

    fn settled(&self, goal: u64) -> bool {
        self.done >= goal && (!self.orbit || self.resolved == self.done)
    }
}

// What a layer is drawn from.
enum Plan {
    Flame(Box<Packed>, f64),
    Orbit(Box<orbit::Packed>, Vec<f32>),
}

#[derive(Default)]
struct Flight {
    busy: bool,
    landed: Vec<u32>,
    finished: Option<Instant>,
    broken: bool,
}

struct Run {
    keys: Vec<String>,
    class: String,
    size: [u32; 2],
    seed: u64,
    layers: Vec<Layer>,
    walkers: u32,
    steps: u32,
    sent: u32,
    batches: u32,
    goal: u64,
    staging: wgpu::Buffer,
    flight: Arc<Mutex<Flight>>,
    started: Instant,
    submitted: Instant,
    fastest: f32,
    took: Vec<(f32, u64)>,
    working: u64,
    spent: f32,
    limit: Option<f32>,
    first: Option<f32>,
    full: Option<f32>,
}

impl Run {
    fn pictured(&self) -> bool {
        self.layers.iter().all(Layer::pictured)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Waiting,
    Drawn,
    Refused,
}

// The chaos game on the graphics card: a picture of the scene within a frame or two, to look at until the
// processor has rendered the exact one.
pub struct Chaos {
    device: wgpu::Device,
    queue: wgpu::Queue,
    failed: Arc<AtomicBool>,
    pipelines: Arc<OnceLock<Pipelines>>,
    largest_buffer: u64,
    largest_side: u32,
    #[cfg(not(target_arch = "wasm32"))]
    waiter: std::sync::mpsc::Sender<wgpu::SubmissionIndex>,
    refused: Option<Vec<String>>,
    longest: Arc<AtomicU32>,
    finest: Option<f64>,
    // Samples a second, the most a batch has shown of layers of these kinds: the first batch of the next run
    // is sized by it.
    speeds: HashMap<String, f64>,
    recipes: Vec<(String, Option<Arc<Recipe>>)>,
    run: Option<Run>,
}

impl Chaos {
    // The graphics card can play the chaos game: it computes, and offers the limits and the format this needs.
    pub fn able(adapter: &wgpu::Adapter, device: &wgpu::Device) -> bool {
        let limits = device.limits();
        let computes = adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);
        let stored = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING;
        computes
            && limits.max_compute_workgroup_size_x >= GROUP
            && limits.max_compute_invocations_per_workgroup >= GROUP
            && limits.max_compute_workgroups_per_dimension >= MOST_WALKERS / GROUP
            && limits.max_storage_buffers_per_shader_stage >= 3
            && limits.max_storage_textures_per_shader_stage >= 1
            && limits.max_uniform_buffer_binding_size >= size_of::<Look>().max(size_of::<Packed>()) as u64
            && adapter
                .get_texture_format_features(LEVELS)
                .allowed_usages
                .contains(stored)
    }

    pub fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ctx: &egui::Context,
    ) -> Option<Self> {
        if !Self::able(adapter, device) {
            return None;
        }
        let limits = device.limits();
        let failed = Arc::new(AtomicBool::new(false));
        let longest = Arc::new(AtomicU32::new(LONGEST));
        let pipelines = Arc::new(OnceLock::new());
        let build = {
            let (device, failed, pipelines, ctx) = (device.clone(), failed.clone(), pipelines.clone(), ctx.clone());
            move || {
                let made = guarded(&device, &failed, &ctx, || Pipelines::new(&device));
                if !failed.load(Ordering::Relaxed) {
                    let _ = pipelines.set(made);
                }
                ctx.request_repaint();
            }
        };
        // A driver takes up to seconds to compile the shaders: the window does not wait for it.
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(build);
        #[cfg(target_arch = "wasm32")]
        build();

        // wgpu tells that work is finished only to a thread that asks: this one sleeps in the driver until then.
        #[cfg(not(target_arch = "wasm32"))]
        let waiter = {
            let (sender, receiver) = std::sync::mpsc::channel::<wgpu::SubmissionIndex>();
            let (device, failed, ctx, longest) = (device.clone(), failed.clone(), ctx.clone(), longest.clone());
            std::thread::spawn(move || {
                while let Ok(index) = receiver.recv() {
                    let wait = wgpu::PollType::Wait {
                        submission_index: Some(index),
                        timeout: Some(std::time::Duration::from_millis(u64::from(
                            longest.load(Ordering::Relaxed),
                        ))),
                    };
                    if let Err(error) = device.poll(wait) {
                        log::warn!("the chaos game on the graphics card is off: {error}");
                        failed.store(true, Ordering::Relaxed);
                        ctx.request_repaint();
                    }
                }
            });
            sender
        };
        Some(Self {
            device: device.clone(),
            queue: queue.clone(),
            failed,
            pipelines,
            largest_buffer: limits.max_buffer_size.min(limits.max_storage_buffer_binding_size),
            largest_side: limits.max_texture_dimension_2d,
            #[cfg(not(target_arch = "wasm32"))]
            waiter,
            refused: None,
            longest,
            finest: None,
            speeds: HashMap::new(),
            recipes: Vec::new(),
            run: None,
        })
    }

    pub fn works(&self) -> bool {
        !self.failed.load(Ordering::Relaxed)
    }

    // The shaders are still being compiled: a run cannot begin yet, and soon can.
    pub fn pending(&self) -> bool {
        self.works() && self.pipelines.get().is_none()
    }

    pub fn keys(&self) -> Option<&[String]> {
        self.run.as_ref().map(|run| run.keys.as_slice())
    }

    pub fn end(&mut self) {
        self.run = None;
    }

    // Seconds the graphics card worked on the run so far, without the waits between its batches.
    pub fn spent(&self) -> f32 {
        self.run.as_ref().map_or(0.0, |run| run.spent)
    }

    // The seconds each batch of the run took and the steps its walkers were allowed.
    pub fn batches(&self) -> &[(f32, u64)] {
        self.run.as_ref().map_or(&[], |run| run.took.as_slice())
    }

    pub fn finished(&self) -> bool {
        self.run.as_ref().is_some_and(|run| run.full.is_some())
    }

    // Has the thread that waits for the graphics card wait for this submission too: what was asked to be told
    // when it is finished is then told.
    pub fn watch(&self, index: wgpu::SubmissionIndex) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.waiter.send(index);
        #[cfg(target_arch = "wasm32")]
        let _ = index;
    }

    // Seconds from the start of the run to its first picture and to the last of its samples.
    pub fn times(&self) -> (Option<f32>, Option<f32>) {
        self.run.as_ref().map_or((None, None), |run| (run.first, run.full))
    }

    pub fn levels(&self) -> Option<([u32; 2], Vec<&wgpu::TextureView>)> {
        let run = self.run.as_ref().filter(|run| run.batches > 0 && run.pictured())?;
        Some((run.size, run.layers.iter().map(|layer| &layer.levels).collect()))
    }

    // True when the graphics card draws this scene: a run for it goes on, or a new one has started. The keys
    // name what the layers' histograms depend on.
    pub fn begin(
        &mut self,
        scene: &Scene,
        options: &RenderOptions,
        keys: &[String],
        asked: Instant,
        limit: Option<f32>,
        ctx: &egui::Context,
    ) -> bool {
        if !self.works() {
            self.run = None;
            return false;
        }
        if self.run.as_ref().is_some_and(|run| run.keys == keys) {
            return true;
        }
        self.run = None;
        if self.refused.as_deref() == Some(keys) {
            return false;
        }
        let pipelines = self.pipelines.clone();
        let Some(pipelines) = pipelines.get() else {
            return false;
        };
        let size = [options.width, options.height];
        let cells = u64::from(size[0]) * u64::from(size[1]);
        let layers = scene.layers.len() as u64;
        if scene.sheets.is_some()
            || scene.layers.is_empty()
            || options.supersample != 1
            || size[0].max(size[1]) > self.largest_side
            || cells * 16 > self.largest_buffer
            || cells * 24 * layers > ROOM
            || scene
                .layers
                .iter()
                .map(|layer| cells * taps(layer.tone(), size))
                .sum::<u64>()
                > MOST_TAPS
        {
            return false;
        }
        let goal = u64::from(options.samples_per_pixel) * cells;
        let weights: Vec<u64> = scene
            .layers
            .iter()
            .map(|layer| layer.flame().map_or(1, weight))
            .collect();
        // Few enough that the unplotted steps of the first batch leave half of what it may take for points.
        let room = (MOST_STEPS * cells).min(MOST_IN_ALL) / layers;
        let heaviest = weights.iter().copied().max().unwrap_or(1);
        let fit = (room / (heaviest * 2 * u64::from(FUSE))).min(u64::from(MOST_WALKERS)) as u32;
        let walkers = ((cells / 16) as u32)
            .min(fit)
            .div_ceil(GROUP)
            .clamp(1, MOST_WALKERS / GROUP)
            * GROUP;
        let mut plans = Vec::new();
        for layer in &scene.layers {
            let plan = match (layer.flame(), layer.fractal()) {
                (Some(flame), _) => {
                    packed(flame, size, self.finest.unwrap_or(FINEST)).map(|(data, plotted)| Plan::Flame(data, plotted))
                }
                // The walkers count the points they plot in 32 bits.
                (_, Some(shown)) if goal < 1 << 31 => self.recipe(shown).and_then(|recipe| {
                    if recipe.work(goal, walkers) > MOST_WORK * cells {
                        return None;
                    }
                    let (data, tables) = recipe.packed(shown, size, goal, walkers, self.finest)?;
                    Some(Plan::Orbit(data, tables))
                }),
                _ => None,
            };
            let Some(plan) = plan else {
                return false;
            };
            plans.push(plan);
        }
        let class = scene
            .layers
            .iter()
            .map(|layer| layer.fractal().map_or("flame", |shown| shown.kind.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        let speed = self.speeds.get(&class).copied().unwrap_or(0.0);
        let first = (f64::from(FIRST_SAMPLES) * cells as f64).max(speed * f64::from(PACE) * 0.5 / layers as f64);
        let (device, failed) = (self.device.clone(), self.failed.clone());
        let made = guarded(&device, &failed, ctx, || {
            let buffer = |label, size: u64, usage| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size,
                    usage,
                    mapped_at_creation: false,
                })
            };
            let (uniform, storage) = (
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                wgpu::BufferUsages::STORAGE,
            );
            let layers = scene
                .layers
                .iter()
                .zip(plans.iter().zip(&weights))
                .map(|(layer, (plan, weight))| {
                    let (flame, plotted, tail) = match plan {
                        Plan::Flame(data, plotted) => {
                            let flame = buffer("fractique flame", size_of::<Packed>() as u64, uniform);
                            self.queue.write_buffer(&flame, 0, bytemuck::bytes_of(&**data));
                            (flame, *plotted, Vec::new())
                        }
                        Plan::Orbit(data, tables) => {
                            let flame = buffer("fractique orbit", size_of::<orbit::Packed>() as u64, uniform);
                            self.queue.write_buffer(&flame, 0, bytemuck::bytes_of(&**data));
                            // A slot of the walkers holds four numbers of a table.
                            let tail: Vec<[f32; 8]> = tables
                                .chunks(4)
                                .map(|four| {
                                    let mut slot = [0f32; 8];
                                    slot[..four.len()].copy_from_slice(four);
                                    slot
                                })
                                .collect();
                            (flame, 1.0, tail)
                        }
                    };
                    let batch = buffer("fractique batch", size_of::<Batch>() as u64, uniform);
                    let kernels = look(layer.tone(), size);
                    let look_buffer = buffer("fractique look", size_of::<Look>() as u64, uniform);
                    self.queue.write_buffer(&look_buffer, 0, bytemuck::bytes_of(&*kernels));
                    let walking = buffer(
                        "fractique walkers",
                        (u64::from(walkers) + tail.len() as u64) * 32,
                        storage | wgpu::BufferUsages::COPY_DST,
                    );
                    if !tail.is_empty() {
                        self.queue
                            .write_buffer(&walking, u64::from(walkers) * 32, bytemuck::cast_slice(&tail));
                    }
                    let histogram = buffer("fractique histogram", cells * 16, storage);
                    let landed = buffer("fractique landed", 16, storage | wgpu::BufferUsages::COPY_SRC);
                    let tiles = buffer(
                        "fractique tiles",
                        u64::from(kernels.size[2]) * u64::from(kernels.size[3]) * 4,
                        storage,
                    );
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
                    let work = Work {
                        walking: self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("fractique chaos"),
                            layout: &pipelines.walking,
                            entries: &[
                                bound(0, &flame),
                                bound(1, &batch),
                                bound(2, &walking),
                                bound(3, &histogram),
                                bound(4, &landed),
                            ],
                        }),
                        looking: self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("fractique levels"),
                            layout: &pipelines.looking,
                            entries: &[
                                bound(0, &look_buffer),
                                bound(1, &histogram),
                                bound(2, &tiles),
                                wgpu::BindGroupEntry {
                                    binding: 3,
                                    resource: wgpu::BindingResource::TextureView(&levels),
                                },
                            ],
                        }),
                        batch,
                        look: look_buffer,
                        landed,
                    };
                    Layer {
                        work: Some(work),
                        levels,
                        kernels,
                        plotted,
                        zoom: layer.camera().zoom,
                        weight: *weight,
                        orbit: matches!(plan, Plan::Orbit(..)),
                        done: 0,
                        resolved: 0,
                    }
                })
                .collect::<Vec<_>>();
            let staging = buffer(
                "fractique landed",
                16 * layers.len() as u64,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            );
            Run {
                keys: keys.to_vec(),
                class,
                size,
                seed: scene.seed,
                layers,
                walkers,
                steps: ((first / f64::from(walkers)).ceil() as u32).max(1),
                sent: 0,
                batches: 0,
                goal,
                staging,
                flight: Arc::default(),
                started: asked,
                submitted: asked,
                fastest: f32::INFINITY,
                took: Vec::new(),
                working: 0,
                spent: 0.0,
                limit,
                first: None,
                full: None,
            }
        });
        if !self.works() {
            return false;
        }
        self.run = Some(made);
        true
    }

    // What the kind worked out from the fractal's parameters, the same for every view of it, or nothing for a
    // fractal the card does not draw.
    fn recipe(&mut self, fractal: &fractique_core::Fractal) -> Option<Arc<Recipe>> {
        let key = Recipe::key(fractal);
        if let Some(at) = self.recipes.iter().position(|(kept, _)| *kept == key) {
            let found = self.recipes.remove(at);
            self.recipes.push(found);
        } else {
            self.recipes.push((key, Recipe::new(fractal).map(Arc::new)));
            if self.recipes.len() > RECIPES {
                self.recipes.remove(0);
            }
        }
        self.recipes.last().and_then(|(_, recipe)| recipe.clone())
    }

    // Takes in the batch that has finished and sends the next one. After Drawn the levels are new and the
    // picture is to be drawn from them again.
    pub fn advance(&mut self, ctx: &egui::Context) -> Step {
        let Some(run) = &mut self.run else {
            return Step::Waiting;
        };
        if self.failed.load(Ordering::Relaxed) {
            self.run = None;
            return Step::Refused;
        }
        {
            let mut flight = run.flight.lock().unwrap_or_else(|e| e.into_inner());
            if flight.busy {
                return Step::Waiting;
            }
            if let Some(at) = flight.finished.take() {
                let mut sparse = false;
                for (layer, counts) in run.layers.iter_mut().zip(flight.landed.chunks_exact(4)) {
                    if !layer.orbit {
                        sparse |= f64::from(counts[0]) < IN_VIEW * layer.plotted * layer.done as f64;
                        continue;
                    }
                    layer.done = u64::from(counts[1]);
                }
                // A card that would take longer than the processor is told to stop: the processor renders. Its
                // own work counts, not the wait for its shaders or for a frame.
                let since = run.started.elapsed().as_secs_f32();
                let took = at.duration_since(run.submitted).as_secs_f32().max(1e-4);
                let worked = run.spent + took;
                let least = run.layers.iter().map(|layer| layer.done).min().unwrap_or(0);
                let slow = least < run.goal
                    && run.limit.is_some_and(|limit| {
                        let share = if least == 0 {
                            1.0
                        } else {
                            run.goal as f32 / least as f32
                        };
                        worked * share > limit
                    });
                let idle = least == 0 && run.batches >= IDLE_BATCHES;
                let hung = took * 1000.0 > self.longest.load(Ordering::Relaxed) as f32;
                if hung {
                    log::warn!("the chaos game on the graphics card is off: a batch took {took:.2} s");
                }
                if flight.broken || sparse || slow || idle || hung {
                    if flight.broken || hung {
                        self.failed.store(true, Ordering::Relaxed);
                    }
                    drop(flight);
                    self.refused = self.run.take().map(|run| run.keys);
                    return Step::Refused;
                }
                if run.pictured() {
                    run.first.get_or_insert(since);
                }
                run.spent += took;
                run.took.push((took, run.working));
                if run.sent > 0 {
                    // Drawing the levels costs the same after a short batch: a batch walks for longer than that
                    // takes.
                    run.fastest = run.fastest.min(took);
                    let pace = PACE.max(2.5 * run.fastest);
                    let next = (run.sent as f32 * pace / took) as u32;
                    let least = run.goal.div_ceil(u64::from(run.walkers) * MOST_BATCHES) as u32;
                    run.steps = next.clamp(run.sent / 4, run.sent.saturating_mul(4)).max(least).max(1);
                    let speed =
                        f64::from(run.sent) * f64::from(run.walkers) * run.layers.len() as f64 / f64::from(took);
                    let most = self.speeds.entry(run.class.clone()).or_insert(0.0);
                    *most = most.max(speed);
                }
                if run.layers.iter().all(|layer| layer.settled(run.goal)) {
                    run.full.get_or_insert(since);
                    run.layers.iter_mut().for_each(|layer| layer.work = None);
                }
            }
        }
        if run.full.is_some() {
            return Step::Waiting;
        }
        let Some(pipelines) = self.pipelines.get() else {
            return Step::Waiting;
        };
        let cells = f64::from(run.size[0]) * f64::from(run.size[1]);
        let (device, failed) = (self.device.clone(), self.failed.clone());
        // Before the submission: a graphics card that is the processor in disguise works inside it.
        run.submitted = Instant::now();
        let (mut sent, mut working) = (0, 0);
        let index = guarded(&device, &failed, ctx, || {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fractique chaos"),
            });
            let (goal, walkers, count) = (run.goal, run.walkers, run.layers.len() as u64);
            for (index, layer) in run.layers.iter_mut().enumerate() {
                let Some(work) = &layer.work else {
                    continue;
                };
                let look = &layer.kernels;
                let resolve = |encoder: &mut wgpu::CommandEncoder, points: u64| {
                    let scale = [(layer.zoom * layer.zoom * cells / points as f64) as f32, 0.0, 0.0, 0.0];
                    self.queue.write_buffer(&work.look, 16, bytemuck::bytes_of(&scale));
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("fractique levels"),
                        timestamp_writes: None,
                    });
                    pass.set_bind_group(0, &work.looking, &[]);
                    pass.set_pipeline(&pipelines.survey);
                    pass.dispatch_workgroups(look.size[2].div_ceil(8), look.size[3].div_ceil(8), 1);
                    pass.set_pipeline(&pipelines.resolve);
                    pass.dispatch_workgroups(look.size[0].div_ceil(8), look.size[1].div_ceil(8), 1);
                };
                if layer.orbit && layer.done > layer.resolved {
                    resolve(&mut encoder, layer.done);
                    layer.resolved = layer.done;
                }
                let left = goal.saturating_sub(layer.done);
                if left == 0 {
                    continue;
                }
                let few = u64::from(walkers);
                let fuse = if run.batches == 0 && !layer.orbit { FUSE } else { 0 };
                let room = (MOST_STEPS * cells as u64).min(MOST_IN_ALL) / (count * few * layer.weight);
                let most = room.saturating_sub(u64::from(fuse)).max(1);
                let steps = (u64::from(run.steps) * u64::from(walkers))
                    .min(left)
                    .div_ceil(few)
                    .min(most);
                let budget = if layer.orbit { (steps * UNSEEN).min(most) } else { steps };
                if !layer.orbit {
                    layer.done += steps * few;
                }
                let batch = Batch {
                    steps: steps as u32,
                    fuse: if run.batches == 0 { FUSE } else { 0 },
                    seed: [run.seed as u32, (run.seed >> 32) as u32],
                    layer: index as u32,
                    walkers: few as u32,
                    budget: budget as u32,
                    spare: 0,
                };
                self.queue.write_buffer(&work.batch, 0, bytemuck::bytes_of(&batch));
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("fractique chaos"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(if layer.orbit { &pipelines.orbit } else { &pipelines.walk });
                    pass.set_bind_group(0, &work.walking, &[]);
                    pass.dispatch_workgroups(few as u32 / GROUP, 1, 1);
                }
                if !layer.orbit {
                    resolve(&mut encoder, layer.done);
                }
                sent = sent.max((steps * few).div_ceil(u64::from(walkers)) as u32);
                working += (budget + u64::from(fuse)) * few * layer.weight;
                encoder.copy_buffer_to_buffer(&work.landed, 0, &run.staging, 16 * index as u64, 16);
            }
            self.queue.submit([encoder.finish()])
        });
        run.sent = sent;
        run.working = working;
        run.batches += 1;
        run.flight.lock().unwrap_or_else(|e| e.into_inner()).busy = true;
        let (flight, staging, ctx) = (run.flight.clone(), run.staging.clone(), ctx.clone());
        run.staging.slice(..).map_async(wgpu::MapMode::Read, move |mapped| {
            let mut flight = flight.lock().unwrap_or_else(|e| e.into_inner());
            match mapped {
                Ok(()) => {
                    match staging.slice(..).get_mapped_range() {
                        Ok(view) => flight.landed = bytemuck::pod_collect_to_vec(&view),
                        Err(_) => flight.broken = true,
                    }
                    staging.unmap();
                }
                Err(_) => flight.broken = true,
            }
            flight.busy = false;
            flight.finished = Some(Instant::now());
            ctx.request_repaint();
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.waiter.send(index);
        #[cfg(target_arch = "wasm32")]
        let _ = index;
        let pictured = run.pictured();
        if !self.works() {
            self.run = None;
            Step::Refused
        } else if pictured {
            Step::Drawn
        } else {
            Step::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::path::PathBuf;

    use fractique_core::random::{Generator, random_scene_of};
    use fractique_core::{Control, fractal, presets};

    use super::*;
    use crate::preview::Preview;

    // Without a browser wgpu answers at once.
    fn ready<T>(future: impl Future<Output = T>) -> Option<T> {
        let mut future = Box::pin(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(out) => Some(out),
            std::task::Poll::Pending => None,
        }
    }

    struct Card {
        device: wgpu::Device,
        queue: wgpu::Queue,
        ctx: egui::Context,
        chaos: Chaos,
        preview: Preview,
    }

    struct Shown;

    impl log::Log for Shown {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Warn
        }

        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                eprintln!("{}: {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }

    fn card() -> Option<Card> {
        if log::set_logger(&Shown).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = ready(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?.ok()?;
        let descriptor = wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        };
        let (device, queue) = ready(adapter.request_device(&descriptor))?.ok()?;
        let ctx = egui::Context::default();
        let chaos = Chaos::new(&adapter, &device, &queue, &ctx)?;
        let preview = Preview::new(&adapter, &device, &queue, &ctx)?;
        while chaos.pending() {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        eprintln!("graphics: {:?}", adapter.get_info());
        // A software adapter compiles a shader when it is first run.
        if adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            chaos.longest.store(20_000, Ordering::Relaxed);
        }
        chaos.works().then_some(Card {
            device,
            queue,
            ctx,
            chaos,
            preview,
        })
    }

    impl Card {
        fn wait(&self) {
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the graphics card answers");
        }

        // The picture the graphics card alone draws of the scene, and the seconds to its first and its last
        // samples.
        fn drawn(&mut self, scene: &Scene, options: &RenderOptions) -> Option<(Vec<u8>, f32, f32)> {
            let keys = vec![format!("{options:?} {}", scene.to_json())];
            self.chaos.end();
            if !self.chaos.begin(scene, options, &keys, Instant::now(), None, &self.ctx) {
                return None;
            }
            let (first, full) = loop {
                if self.chaos.advance(&self.ctx) == Step::Refused {
                    return None;
                }
                if let (Some(first), Some(full)) = self.chaos.times() {
                    break (first, full);
                }
                self.wait();
            };
            let (size, levels) = self.chaos.levels()?;
            if !self.preview.draw_levels(scene, size, &levels, &self.ctx) {
                return None;
            }
            let (picture, _) = self.preview.views()?;
            let stride = (size[0] * 4).div_ceil(256) * 256;
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(stride) * u64::from(size[1]),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: picture.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(size[1]),
                    },
                },
                wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
            );
            self.queue.submit([encoder.finish()]);
            buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            self.wait();
            let rows = buffer.slice(..).get_mapped_range().ok()?;
            let rgba = rows
                .chunks(stride as usize)
                .flat_map(|row| row[..size[0] as usize * 4].iter().copied())
                .collect();
            self.chaos.end();
            Some((rgba, first, full))
        }
    }

    const BLOCK: usize = 8;

    // How far two pictures are apart, on the scale of 0 to 255: the mean and the largest difference between the
    // averages of their 8×8 blocks, so that the noise of single pixels does not count.
    fn apart(one: &[u8], other: &[u8], width: usize, height: usize) -> (f64, f64) {
        let (across, down) = (width / BLOCK, height / BLOCK);
        let (mut sum, mut largest) = (0.0, 0f64);
        for block in 0..across * down {
            let (left, top) = (block % across * BLOCK, block / across * BLOCK);
            for channel in 0..3 {
                let mut difference = 0i64;
                for y in top..top + BLOCK {
                    for x in left..left + BLOCK {
                        let at = (y * width + x) * 4 + channel;
                        difference += i64::from(one[at]) - i64::from(other[at]);
                    }
                }
                let difference = difference.abs() as f64 / (BLOCK * BLOCK) as f64;
                sum += difference;
                largest = largest.max(difference);
            }
        }
        (sum / (across * down * 3) as f64, largest)
    }

    fn options(width: u32, height: u32) -> RenderOptions {
        RenderOptions {
            width,
            height,
            samples_per_pixel: 32,
            supersample: 1,
        }
    }

    #[test]
    fn flames_are_packed_as_the_shader_reads_them() {
        assert_eq!(size_of::<Packed>(), 80 + 65 * 80 + 256 * 32 + 1024 + 512);
        let scene = presets::get("solar-mandala").expect("the preset exists");
        let flame = scene.layers[0].flame().expect("the preset is a flame");
        let (data, plotted) = packed(flame, [640, 360], FINEST).expect("the flame is one the graphics card draws");
        assert_eq!(data.counts[0] as usize, flame.transforms.len());
        assert_eq!(data.counts[1], flame.symmetry.unsigned_abs());
        assert!(plotted > 0.0 && plotted <= 1.0);
        let weights: f64 = flame.transforms.iter().map(|transform| transform.weight).sum();
        assert_eq!(data.xfs[flame.transforms.len() - 1].after[3], weights as f32);

        let mut deep = flame.clone();
        deep.camera.zoom = 1.0e7;
        assert!(packed(&deep, [640, 360], FINEST).is_none());
    }

    #[test]
    fn the_kernels_are_those_of_the_density_estimation() {
        let scene = presets::get("aurora-spiral").expect("the preset exists");
        let look = look(scene.layers[0].tone(), [3840, 2160]);
        assert_eq!(look.widths[0][1], 17);
        assert_eq!(look.widths[8][0], 4);
        assert_eq!(look.widths[1023][3], 0);
        assert_eq!(look.kernels[17][0], 9.0);
        assert_eq!(look.kernels[0], [0.0, (1.0 / 0.18f64) as f32, 1.0, 0.0]);
        assert_eq!(look.size[2..], [240, 135]);
    }

    #[test]
    fn the_graphics_card_draws_what_the_processor_renders() {
        let Some(mut card) = card() else {
            eprintln!("no graphics card with compute shaders: nothing is compared");
            return;
        };
        let options = options(320, 200);
        for name in ["aurora-spiral", "solar-mandala", "ember-vortex"] {
            let scene = presets::get(name).expect("the preset exists");
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let (quick, _, _) = card.drawn(&scene, &options).expect("the graphics card draws it");
            let (mean, largest) = apart(&quick, &exact.rgba, 320, 200);
            assert!(
                mean < 3.0 && largest < 40.0,
                "{name}: {mean:.2} on average, {largest:.1} at most"
            );
        }
    }

    // Moves the flame away from the origin, where 32-bit numbers have fewer digits left for a pixel, and prints
    // how far the graphics card's picture is from the processor's: where FINEST comes from.
    #[test]
    #[ignore = "a report, not a check"]
    fn how_far_single_precision_reaches() {
        let mut card = card().expect("a graphics card with compute shaders");
        card.chaos.finest = Some(f64::INFINITY);
        let options = options(960, 540);
        println!("| Cells a unit × distance | Mean | Largest |");
        for away in [0.0, 300.0, 1000.0, 3000.0, 10_000.0, 30_000.0, 100_000.0] {
            let mut scene = presets::get("aurora-spiral").expect("the preset exists");
            let flame = scene.layers[0].flame_mut().expect("the preset is a flame");
            flame.final_transform = Some(Transform {
                weight: 1.0,
                color: 0.0,
                color_speed: 0.0,
                opacity: 1.0,
                affine: [1.0, 0.0, away, 0.0, 1.0, away * 0.7],
                post: None,
                variations: vec![fractique_core::Variation::new(Kind::Linear, 1.0)],
            });
            flame.camera.center[0] += away;
            flame.camera.center[1] += away * 0.7;
            let per_unit = flame.camera.zoom * 540.0 / 4.0;
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let Some((quick, _, _)) = card.drawn(&scene, &options) else {
                println!("| {:.1e} | not drawn | |", per_unit * away);
                continue;
            };
            let (mean, largest) = apart(&quick, &exact.rgba, 960, 540);
            println!("| {:.1e} | {mean:.2} | {largest:.1} |", per_unit * away);
            if let Some(folder) = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from) {
                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                save(&folder.join(format!("away-{away}.png")), &both, 960, 1080);
            }
        }
    }

    #[test]
    fn a_view_with_little_of_the_flame_is_left_to_the_processor() {
        let Some(mut card) = card() else {
            return;
        };
        let mut scene = presets::get("aurora-spiral").expect("the preset exists");
        let camera = scene.layers[0].camera_mut();
        (camera.center, camera.zoom) = ([-0.9, 0.56], 24.0);
        assert!(card.drawn(&scene, &options(320, 200)).is_none());
        let request = fractique_core::wallpaper::Request {
            layout: None,
            material: None,
            ratio: 0.625,
            dark: true,
            hue: None,
        };
        let wallpaper = fractique_core::wallpaper::random_wallpaper(3, &request);
        assert!(card.drawn(&wallpaper, &options(320, 200)).is_none());
    }

    // A flame of two plain transforms and one that is the variation alone, framed by the randomizer's camera.
    fn flame_of(kind: Kind) -> Scene {
        let mut scene = presets::get("aurora-spiral").expect("the preset exists");
        let flame = scene.layers[0].flame_mut().expect("the preset is a flame");
        let plain = |color: f64, affine: [f64; 6], kind: Kind| Transform {
            weight: 1.0,
            color,
            color_speed: 0.5,
            opacity: 1.0,
            affine,
            post: None,
            variations: vec![fractique_core::Variation::new(kind, 1.0)],
        };
        flame.transforms = vec![
            plain(0.0, [0.6, -0.3, 0.4, 0.3, 0.6, -0.2], Kind::Linear),
            plain(0.5, [-0.5, 0.2, -0.5, 0.2, 0.55, 0.3], Kind::Linear),
            plain(1.0, [0.8, 0.3, 0.1, -0.3, 0.8, 0.2], kind),
        ];
        (flame.symmetry, flame.final_transform) = (1, None);
        if let Some((camera, _)) = fractique_core::random::fit_camera(flame) {
            flame.camera = camera;
        }
        scene.name = kind.name();
        scene
    }

    // FRACTIQUE_COMPARE=folder: also writes each flame as a scene file, for the app's FRACTIQUE_SHOW=gpu.
    #[test]
    #[ignore = "a report, not a check"]
    fn every_variation_on_the_graphics_card_beside_the_processor() {
        let mut card = card().expect("a graphics card with compute shaders");
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let options = RenderOptions {
            samples_per_pixel: 200,
            ..options(640, 360)
        };
        println!("| Variation | Mean | Largest |");
        println!("| --- | --- | --- |");
        for kind in Kind::ALL {
            let scene = flame_of(kind);
            if let Some(folder) = &folder {
                std::fs::write(folder.join(format!("{}.json", kind.name())), scene.to_json()).expect("it is written");
            }
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let Some((quick, _, _)) = card.drawn(&scene, &options) else {
                println!("| {} | not drawn | |", kind.name());
                continue;
            };
            let (mean, largest) = apart(&quick, &exact.rgba, 640, 360);
            println!("| {} | {mean:.2} | {largest:.1} |", kind.name());
            if let Some(folder) = &folder {
                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                save(&folder.join(format!("{}.png", kind.name())), &both, 640, 720);
            }
        }
    }

    #[test]
    fn the_graphics_card_draws_the_orbits_the_processor_renders() {
        let Some(mut card) = card() else {
            eprintln!("no graphics card with compute shaders: nothing is compared");
            return;
        };
        let options = options(320, 200);
        for name in ["clifford-classic", "de_jong-classic", "lorenz-butterfly"] {
            let scene = fractal::preset(name).expect("the preset exists");
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let (quick, _, _) = card.drawn(&scene, &options).expect("the graphics card draws it");
            let (mean, largest) = apart(&quick, &exact.rgba, 320, 200);
            assert!(
                mean < 3.0 && largest < 40.0,
                "{name}: {mean:.2} on average, {largest:.1} at most"
            );
        }
        let mut both = fractal::preset("clifford-classic").expect("the preset exists");
        both.layers
            .extend(presets::get("aurora-spiral").expect("the preset exists").layers);
        let exact = fractique_core::render(&both, &options, &Control::default()).expect("it renders");
        let (quick, _, _) = card
            .drawn(&both, &options)
            .expect("the graphics card draws a flame over an orbit");
        let (mean, largest) = apart(&quick, &exact.rgba, 320, 200);
        assert!(
            mean < 3.0 && largest < 40.0,
            "{mean:.2} on average, {largest:.1} at most"
        );

        for kind in ["mandelbrot", "hopalong", "buddhabrot", "gumowski_mira"] {
            let scene = fractal::random_scene(kind, 1).expect("the kind is listed");
            assert!(card.drawn(&scene, &options).is_none(), "{kind}");
        }
    }

    // FRACTIQUE_COMPARE=folder cargo test --release -p fractique-app --lib every_orbit -- --ignored --nocapture
    // writes the table of differences for the presets and ten random scenes of every kind of orbit, and for each
    // the picture of the graphics card above the processor's. FRACTIQUE_COMPARE_SIZE, _SAMPLES and _ONLY as above.
    #[test]
    #[ignore = "a report, not a check"]
    fn every_orbit_on_the_graphics_card_beside_the_processor() {
        let mut card = card().expect("a graphics card with compute shaders");
        let samples = std::env::var("FRACTIQUE_COMPARE_SAMPLES")
            .ok()
            .and_then(|samples| samples.parse().ok())
            .unwrap_or(32);
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let size: Vec<u32> = std::env::var("FRACTIQUE_COMPARE_SIZE")
            .unwrap_or_else(|_| "960x540".to_owned())
            .split('x')
            .filter_map(|side| side.parse().ok())
            .collect();
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        let options = RenderOptions {
            samples_per_pixel: samples,
            ..options(size[0], size[1])
        };
        let mut scenes: Vec<(String, Scene)> = Vec::new();
        for spec in fractal::all().filter(|spec| spec.is_density()) {
            for preset in spec.presets {
                let name = format!("{}-{}", spec.name, preset.name);
                scenes.push((name.clone(), fractal::preset(&name).expect("the preset exists")));
            }
            for seed in 1..=10 {
                let scene = fractal::random_scene(spec.name, seed).expect("the kind is listed");
                scenes.push((format!("{}-random-{seed}", spec.name), scene));
            }
        }
        println!("| Scene | Mean | Largest | First picture, s | All samples, s | Processor, s |");
        println!("| --- | --- | --- | --- | --- | --- |");
        for (name, scene) in scenes {
            if only.as_ref().is_some_and(|only| !name.contains(only.as_str())) {
                continue;
            }
            let started = Instant::now();
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let slow = started.elapsed().as_secs_f32();
            let Some((quick, first, full)) = card.drawn(&scene, &options) else {
                println!("| {name} | not drawn | | | | {slow:.2} |");
                continue;
            };
            let (mean, largest) = apart(&quick, &exact.rgba, size[0] as usize, size[1] as usize);
            println!("| {name} | {mean:.2} | {largest:.1} | {first:.3} | {full:.3} | {slow:.2} |");
            if let Some(folder) = &folder {
                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                save(&folder.join(format!("{name}.png")), &both, size[0], size[1] * 2);
            }
        }
    }

    // Magnifies a part of each attractor that holds many of its points and prints how far the graphics card's
    // picture is from the processor's: where the limits of orbit.rs come from.
    #[test]
    #[ignore = "a report, not a check"]
    fn how_far_single_precision_reaches_into_an_orbit() {
        let mut card = card().expect("a graphics card with compute shaders");
        card.chaos.finest = Some(f64::INFINITY);
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let options = RenderOptions {
            samples_per_pixel: 400,
            ..options(320, 200)
        };
        println!("| Kind | Cells a unit × distance | Points in view | Mean | Largest |");
        println!("| --- | --- | --- | --- | --- |");
        for name in ["clifford-classic", "de_jong-classic", "lorenz-butterfly"] {
            let whole = fractal::preset(name).expect("the preset exists");
            let wide = RenderOptions {
                samples_per_pixel: 8,
                ..options
            };
            let picture = fractique_core::render(&whole, &wide, &Control::default()).expect("it renders");
            let brightest = (0..320 * 200)
                .max_by_key(|at| {
                    picture.rgba[at * 4..at * 4 + 3]
                        .iter()
                        .map(|c| u32::from(*c))
                        .sum::<u32>()
                })
                .expect("the picture has pixels");
            let camera = whole.layers[0].camera().clone();
            let per_unit = camera.zoom * 200.0 / 4.0;
            let middle = [
                camera.center[0] + ((brightest % 320) as f64 + 0.5 - 160.0) / per_unit,
                camera.center[1] - ((brightest / 320) as f64 + 0.5 - 100.0) / per_unit,
            ];
            for reach in [1.0e4, 1.0e5, 4.0e5, 1.0e6, 4.0e6] {
                let mut scene = whole.clone();
                let far = middle[0].abs().max(middle[1].abs());
                let camera = scene.layers[0].camera_mut();
                (camera.center, camera.zoom) = (middle, reach / far / 50.0);
                let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
                let lit = exact
                    .rgba
                    .chunks_exact(4)
                    .filter(|pixel| pixel[0].max(pixel[1]) > 24)
                    .count();
                let Some((quick, _, _)) = card.drawn(&scene, &options) else {
                    println!("| {name} | {reach:.0e} | {lit} | not drawn | |");
                    continue;
                };
                let (mean, largest) = apart(&quick, &exact.rgba, 320, 200);
                println!("| {name} | {reach:.0e} | {lit} | {mean:.2} | {largest:.1} |");
                if let Some(folder) = &folder {
                    let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                    save(&folder.join(format!("deep-{name}-{reach:.0e}.png")), &both, 320, 400);
                }
            }
        }
    }

    fn save(path: &std::path::Path, rgba: &[u8], width: u32, height: u32) {
        let file = std::fs::File::create(path).expect("the folder exists");
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("the file is written");
        writer.write_image_data(rgba).expect("the file is written");
    }

    // FRACTIQUE_COMPARE=folder cargo test --release -p fractique-app --lib every_scene -- --ignored --nocapture
    // writes the table of differences, and for every scene the picture of the graphics card above the processor's.
    #[test]
    #[ignore = "a report, not a check"]
    fn every_scene_on_the_graphics_card_beside_the_processor() {
        let mut card = card().expect("a graphics card with compute shaders");
        let samples = std::env::var("FRACTIQUE_COMPARE_SAMPLES")
            .ok()
            .and_then(|samples| samples.parse().ok())
            .unwrap_or(32);
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let size: Vec<u32> = std::env::var("FRACTIQUE_COMPARE_SIZE")
            .unwrap_or_else(|_| "960x540".to_owned())
            .split('x')
            .filter_map(|side| side.parse().ok())
            .collect();
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        let options = RenderOptions {
            samples_per_pixel: samples,
            ..options(size[0], size[1])
        };
        let mut scenes: Vec<(String, Scene)> = presets::names()
            .into_iter()
            .filter_map(|name| Some((name.to_owned(), presets::get(name)?)))
            .collect();
        for (generator, label) in [(Generator::First, "random"), (Generator::Second, "random2")] {
            for seed in 1..=20 {
                scenes.push((format!("{label}-{seed}"), random_scene_of(generator, seed)));
            }
        }
        println!("| Scene | Mean | Largest | First picture, s | All samples, s | Processor, s |");
        println!("| --- | --- | --- | --- | --- | --- |");
        for (name, scene) in scenes {
            if only.as_ref().is_some_and(|only| !name.contains(only.as_str())) {
                continue;
            }
            let started = Instant::now();
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let slow = started.elapsed().as_secs_f32();
            let Some((quick, first, full)) = card.drawn(&scene, &options) else {
                println!("| {name} | not drawn | | | | {slow:.2} |");
                continue;
            };
            let (mean, largest) = apart(&quick, &exact.rgba, size[0] as usize, size[1] as usize);
            println!("| {name} | {mean:.2} | {largest:.1} | {first:.3} | {full:.3} | {slow:.2} |");
            if let Some(folder) = &folder {
                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                save(&folder.join(format!("{name}.png")), &both, size[0], size[1] * 2);
            }
        }
    }
}
