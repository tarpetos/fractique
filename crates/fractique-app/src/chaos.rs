use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_wgpu::wgpu;
use fractique_core::scene::Transform;
use fractique_core::{Flame, Kind, RenderOptions, Scene, Tone, fractal, math, palette, presets};
use web_time::Instant;

use crate::deep::{self, Stage};
use crate::field::{self, Batched, Chain, Fields, Planned};
use crate::orbit::{self, Recipe};
use crate::pace;
use crate::preview::{LEVELS, guarded};
use crate::sheets;

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
// fractique-core's deep.rs: a view magnified more than this may hold little of the flame.
const WHOLE: f64 = 2.0;
// The steps of the chaos game the particles of a deep view take in one submission, at most.
const MOST_DEEP: u64 = 1 << 28;
// So many scenes the card gave up are remembered: they are not tried again.
const REFUSED: usize = 8;
// Histograms and levels of one run, in bytes; half of it for a picture that is drawn with a margin, whose
// histogram is a quarter of the card's largest buffer at the most: a margin is given up before memory is short.
const ROOM: u64 = 1 << 30;
// Cells a unit, times the distance of the view from the origin: above it 32-bit coordinates show as a grid.
const FINEST: f64 = 4.0e5;
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
const MOST_TAPS: u64 = 1 << 34;
// An orbit whose walkers would take more steps than this many a pixel in all is the processor's.
const MOST_WORK: u64 = 512;
// A submission that takes longer than this many milliseconds ends the card's part in the picture.
pub(crate) const LONGEST: u32 = 500;
// A scene of flames alone is walked as it was before the bounds: its batches grow with the card's pace, and it is
// given this many milliseconds. A submission of any scene that takes longer ends the card's part for the session.
const PATIENT: u32 = 20_000;
// A run whose walkers have plotted nothing after this many seconds of the card's work is given up.
const IDLE: f32 = 2.5;
const RECIPES: usize = 8;
// The card's work is cut into units of about this many seconds, sent one after the other while the frame's slice
// lasts: a unit that takes twice as long costs the frame a millisecond, not a slice. A unit and the way back from
// it took 0.25 ms on an RTX 4070, so smaller ones would be mostly that.
const UNIT: f32 = 0.0015;
const DEEP_UNIT: f32 = 0.003;
// Milliseconds without a word of the card after which a picture on its way counts as stalled: ten times the
// longest a batch of a bounded scene may take.
const SILENT: u32 = 10 * LONGEST;
// The scenes of the small pictures that run every shader once, and what their run is known by.
const WARM: [&str; 2] = ["aurora-spiral", "clifford-classic"];
const WARM_KEY: &str = "warm";

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
    // Unplotted steps of walkers that are on their way already.
    idle: u32,
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
fn packed(flame: &Flame, [width, height]: [u32; 2], margin: [u32; 2], finest: f64) -> Option<(Box<Packed>, f64)> {
    if flame.transforms.is_empty() || flame.transforms.len() > TRANSFORMS {
        return None;
    }
    let camera = &flame.camera;
    let per_unit = camera.zoom * f64::from(width.min(height)) / 4.0;
    let [across, down] = [width + 2 * margin[0], height + 2 * margin[1]];
    let far = camera.center[0].abs().max(camera.center[1].abs()) + 0.5 * f64::from(across.max(down)) / per_unit;
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
    data.scale = [per_unit as f32, up as f32, across as f32, down as f32];
    data.extent = [across, down, margin[0], margin[1]];
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

// The kernels of fractique-core's density estimation for a view of the size `inner`, in a picture of this size.
fn look(tone: &Tone, [width, height]: [u32; 2], inner: [u32; 2]) -> Box<Look> {
    let (spread, top) = widest(tone, inner);
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

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &str,
    entry_point: &str,
) -> wgpu::ComputePipeline {
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
}

// The shaders of a deep view: compiled after the others, which a picture of a whole flame does not wait for.
struct Particles {
    pilot: wgpu::ComputePipeline,
    refill: wgpu::ComputePipeline,
    sweep: wgpu::ComputePipeline,
    mark: wgpu::ComputePipeline,
}

impl Particles {
    fn new(device: &wgpu::Device, walking: &wgpu::BindGroupLayout) -> Self {
        Self {
            pilot: pipeline(device, walking, deep::SHADER, "pilot"),
            refill: pipeline(device, walking, deep::SHADER, "refill"),
            sweep: pipeline(device, walking, deep::SHADER, "sweep"),
            mark: pipeline(device, walking, deep::SHADER, "mark"),
        }
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
        let flame = concat!(include_str!("flame.wgsl"), include_str!("chaos.wgsl"));
        Self {
            walk: pipeline(device, &walking, flame, "walk"),
            orbit: pipeline(device, &walking, orbit::SHADER, "walk"),
            survey: pipeline(device, &looking, include_str!("levels.wgsl"), "survey"),
            resolve: pipeline(device, &looking, include_str!("levels.wgsl"), "resolve"),
            walking,
            looking,
        }
    }
}

// What a layer needs while its samples are plotted: with the last of them its histogram and walkers are freed.
struct Work {
    flame: wgpu::Buffer,
    histogram: wgpu::Buffer,
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
    // A magnified view of a flame is shown once its first batch has told that enough of its points landed in the
    // picture. When too few did, the view is drawn from particles: the plan of that is here.
    checked: bool,
    sparse: bool,
    landed: u64,
    deep: Option<deep::Plan>,
    // The points the layer plots in all. A deep view plots them all into its picture: with a margin, which is
    // drawn behind a finished picture of the view alone, the view has its share of them and no more.
    goal: u64,
    // The levels are being drawn, a strip a submission, after one that looks the tiles over: the next row (none
    // before the tiles are looked over), the scale, and the samples the levels are of.
    strip: Option<(Option<u32>, f64, u64)>,
}

impl Layer {
    fn told(&self) -> bool {
        self.orbit || self.deep.is_some()
    }

    fn pictured(&self) -> bool {
        self.resolved > 0 && (self.told() || self.checked)
    }

    fn settled(&self) -> bool {
        self.done >= self.goal && self.resolved == self.done && (self.told() || self.checked)
    }

    // The numbers the card tells of the layer after a batch.
    fn told_in(&self) -> usize {
        self.deep
            .as_ref()
            .map_or(4, |plan| deep::HEAD + plan.particles as usize)
    }
}

// The steps a walker of the next batch, after one of `sent` plotted and `fused` unplotted steps a walker that took
// so long. Every run begins with one plotted step, whatever other scenes have shown: a step of one flame costs
// twenty times a step of another. That first batch is mostly unplotted steps, so the next is sized by all of its
// work. An unplotted step writes nothing and is taken for a third of a plotted one: taken for a whole one, the
// second batch of a flame at 3840 × 2160 took 12 to 18 ms of a slice of 8.3.
fn following(sent: u32, fused: u32, took: f32, slice: f32) -> u32 {
    let worked = f64::from(sent) + f64::from(fused) / 3.0;
    (pace::next(worked, took, slice).floor() as u32).max(1)
}

// What a run needs of the card, for the frame and for the thread that hears of a finished unit alike.
#[derive(Clone)]
struct Parts {
    device: wgpu::Device,
    queue: wgpu::Queue,
    failed: Arc<AtomicBool>,
    pipelines: Arc<OnceLock<Pipelines>>,
    particles: Arc<OnceLock<Option<Particles>>>,
    longest: Arc<AtomicU32>,
    #[cfg(not(target_arch = "wasm32"))]
    waiter: std::sync::mpsc::Sender<wgpu::SubmissionIndex>,
    #[cfg(test)]
    plain: Option<f64>,
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
    bounded: bool,
    // The picture, and the view in its middle: the picture is the view with a margin around it.
    size: [u32; 2],
    inner: [u32; 2],
    seed: u64,
    layers: Vec<Layer>,
    walkers: u32,
    steps: u32,
    sent: u32,
    // The steps every walker of the batch on its way takes unplotted, besides those.
    fused: u32,
    batches: u32,
    staging: wgpu::Buffer,
    flight: Arc<Mutex<Flight>>,
    started: Instant,
    submitted: Instant,
    // The batch in flight draws levels only: its time sizes the next strip, not the next walk.
    resolving: bool,
    rows: u32,
    // When the frame opened the slice the units are sent in, and the seconds it allowed then.
    opened: Instant,
    allowed: f32,
    // What a unit sent from the end of another has come to, for the frame to take.
    told: Step,
    // The run is over: its picture is the processor's, and is not tried again when this is true.
    over: Option<bool>,
    stopped: bool,
    // The most a unit ended past its frame's slice, in seconds.
    overrun: f32,
    // When the card was last heard of: a unit of this run landed, or the run began.
    heard: Instant,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
    // Nothing when the card refused the shaders of a deep view: such a view is then the processor's.
    particles: Arc<OnceLock<Option<Particles>>>,
    largest_buffer: u64,
    largest_side: u32,
    #[cfg(not(target_arch = "wasm32"))]
    waiter: std::sync::mpsc::Sender<wgpu::SubmissionIndex>,
    refused: Vec<Vec<String>>,
    longest: Arc<AtomicU32>,
    finest: Option<f64>,
    // A test plays the plain chaos game for a view that holds little of the flame, to see what a deep view of it
    // should show, with the brightness of the deep view.
    #[cfg(test)]
    plain: Option<f64>,
    recipes: Vec<(String, Option<Arc<Recipe>>)>,
    // The run is the frame's and the thread's that hears of a finished unit: what the frame reads of it without
    // asking is kept beside it.
    run: Option<Arc<Mutex<Run>>>,
    run_keys: Option<Vec<String>>,
    run_levels: Option<([u32; 2], Vec<wgpu::TextureView>)>,
    run_margin: [u32; 2],
    // The finished picture of the view alone, while the run draws the same view again with a margin around it and
    // after: the screen's picture is this one until the wider one is whole, and the view in its middle after.
    base: Option<Base>,
    // How many of the small pictures that run every shader once have been drawn: a card works a shader out when
    // it first runs it (7 ms on an RTX 4070), and that is not left to the first unit of the first picture.
    warmed: usize,
    fields: Fields,
    pixels: Option<Pixels>,
    // The batches of a picture drawn pixel by pixel that has ended: the next one sends its first after the last.
    draining: Option<Chain>,
}

struct Base {
    size: [u32; 2],
    levels: Vec<wgpu::TextureView>,
    times: (Option<f32>, Option<f32>),
}

// A scene of fractals drawn pixel by pixel: its batches follow one another without the window.
struct Pixels {
    keys: Vec<String>,
    heard: Instant,
    size: [u32; 2],
    levels: Vec<wgpu::TextureView>,
    chain: Chain,
    report: field::Report,
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
        let particles = Arc::new(OnceLock::new());
        let build = {
            let (device, failed, pipelines, ctx) = (device.clone(), failed.clone(), pipelines.clone(), ctx.clone());
            let particles = particles.clone();
            move || {
                let made = guarded(&device, &failed, &ctx, || Pipelines::new(&device));
                if !failed.load(Ordering::Relaxed) {
                    let _ = pipelines.set(made);
                }
                ctx.request_repaint();
                if let Some(pipelines) = pipelines.get() {
                    let refused = Arc::new(AtomicBool::new(false));
                    let made = guarded(&device, &refused, &ctx, || Particles::new(&device, &pipelines.walking));
                    let _ = particles.set((!refused.load(Ordering::Relaxed)).then_some(made));
                    ctx.request_repaint();
                }
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
                    let wait = |longest: u32| {
                        device.poll(wgpu::PollType::Wait {
                            submission_index: Some(index.clone()),
                            timeout: Some(std::time::Duration::from_millis(u64::from(longest))),
                        })
                    };
                    // A submission that is late costs its picture, which is given up when it lands; one that
                    // does not land at all costs the session.
                    let landed = match wait(longest.load(Ordering::Relaxed)) {
                        Err(wgpu::PollError::Timeout) => wait(PATIENT),
                        landed => landed,
                    };
                    if let Err(error) = landed {
                        log::warn!("the chaos game on the graphics card is off: {error}");
                        failed.store(true, Ordering::Relaxed);
                        ctx.request_repaint();
                    }
                }
            });
            sender
        };
        // A thread of its own sleeps in the driver until the window's own frame is finished: what the card took
        // for it is what a picture may not have of a frame.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (sender, frames) = std::sync::mpsc::channel::<(wgpu::SubmissionIndex, Instant)>();
            let device = device.clone();
            std::thread::spawn(move || {
                while let Ok((index, sent)) = frames.recv() {
                    let waited = device.poll(wgpu::PollType::Wait {
                        submission_index: Some(index),
                        timeout: Some(std::time::Duration::from_millis(u64::from(LONGEST))),
                    });
                    if waited.is_ok() {
                        pace::FRAME_ON_CARD.store((sent.elapsed().as_micros() as u32).max(1), Ordering::Relaxed);
                    }
                }
            });
            let sender = Mutex::new(sender);
            let _ = eframe::egui_wgpu::AFTER_FRAME.set(Box::new(move |index| {
                let sender = sender.lock().unwrap_or_else(|e| e.into_inner());
                let _ = sender.send((index, Instant::now()));
            }));
        }
        Some(Self {
            fields: Fields::new(adapter, device, queue, failed.clone()),
            pixels: None,
            draining: None,
            device: device.clone(),
            queue: queue.clone(),
            failed,
            pipelines,
            particles,
            largest_buffer: limits.max_buffer_size.min(limits.max_storage_buffer_binding_size),
            largest_side: limits.max_texture_dimension_2d,
            #[cfg(not(target_arch = "wasm32"))]
            waiter,
            refused: Vec::new(),
            longest,
            finest: None,
            #[cfg(test)]
            plain: None,
            recipes: Vec::new(),
            run: None,
            run_keys: None,
            run_levels: None,
            run_margin: [0; 2],
            base: None,
            warmed: 0,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn waiter(&self) -> field::Waiter {
        self.waiter.clone()
    }

    #[cfg(target_arch = "wasm32")]
    fn waiter(&self) -> field::Waiter {}

    // A test draws views deeper than 32-bit floats reach, to measure where they stop.
    #[cfg(test)]
    pub(crate) fn unbound(&mut self, unbounded: bool) {
        self.fields.unbounded = unbounded;
    }

    pub fn works(&self) -> bool {
        !self.failed.load(Ordering::Relaxed)
    }

    // The shaders are still being compiled: a run cannot begin yet, and soon can.
    pub fn pending(&self) -> bool {
        self.works() && (self.pipelines.get().is_none() || self.fields.pending() || self.warmed < WARM.len())
    }

    // Draws the small pictures that run the shaders of flames, of orbits and of the levels once, one unit after
    // the other like any picture: nothing else is begun meanwhile. Asked for by every frame until it is done.
    pub fn warming(&self) -> bool {
        self.works() && self.warmed < WARM.len() && self.pipelines.get().is_some()
    }

    pub fn warm(&mut self, ctx: &egui::Context, [width, height]: [u32; 2]) {
        if !self.warming() {
            return;
        }
        if self.keys() != Some(&[WARM_KEY.to_owned()]) {
            let scene = presets::get(WARM[self.warmed]).or_else(|| fractal::preset(WARM[self.warmed]));
            // At the size of the picture that follows: the card gives its memory out slowly the first time.
            let options = RenderOptions {
                width: width.clamp(64, self.largest_side),
                height: height.clamp(64, self.largest_side),
                samples_per_pixel: 1,
                supersample: 1,
            };
            let begun = scene.is_some_and(|scene| {
                self.started(
                    &scene,
                    &options,
                    [0; 2],
                    &[WARM_KEY.to_owned()],
                    Instant::now(),
                    None,
                    ctx,
                )
            });
            if !begun {
                self.warmed += 1;
                ctx.request_repaint();
                return;
            }
        }
        if self.advance(ctx) == Step::Refused || self.finished() {
            self.end();
            self.refused.retain(|keys| keys.as_slice() != [WARM_KEY.to_owned()]);
            self.warmed += 1;
            ctx.request_repaint();
        }
    }

    pub fn keys(&self) -> Option<&[String]> {
        let pixels = self.pixels.as_ref().map(|pixels| pixels.keys.as_slice());
        pixels.or(self.run_keys.as_deref())
    }

    pub fn end(&mut self) {
        if let Some(run) = self.run.take() {
            run.lock().unwrap_or_else(|e| e.into_inner()).stopped = true;
        }
        (self.run_keys, self.run_levels, self.run_margin) = (None, None, [0; 2]);
        self.base = None;
        if let Some(pixels) = self.pixels.take() {
            pixels.chain.stop();
            if pixels.chain.busy() {
                self.draining = Some(pixels.chain);
            }
        }
    }

    // Of a picture drawn pixel by pixel: the seconds its longest batch took, and how many batches there were.
    pub fn slowest(&self) -> Option<(f32, u32)> {
        let report = self.pixels.as_ref()?.report;
        Some((report.slowest, report.batches))
    }

    // Seconds the graphics card worked on the run so far, without the waits between its batches.
    pub fn spent(&self) -> f32 {
        let pixels = self.pixels.as_ref().map(|pixels| pixels.report.spent);
        pixels.unwrap_or(self.read(|run| run.spent).unwrap_or(0.0))
    }

    // For a test run's trace: the batches so far, the milliseconds the last and the longest took, and all of them.
    pub fn load(&self) -> (usize, f32, f32, f32) {
        if let Some(pixels) = &self.pixels {
            let report = pixels.report;
            return (
                report.batches as usize,
                0.0,
                report.slowest * 1000.0,
                report.spent * 1000.0,
            );
        }
        let took = self.batches();
        let last = took.last().map_or(0.0, |(seconds, _)| *seconds);
        let worst = took.iter().map(|(seconds, _)| *seconds).fold(0.0, f32::max);
        let spent: f32 = took.iter().map(|(seconds, _)| *seconds).sum();
        (took.len(), last * 1000.0, worst * 1000.0, spent * 1000.0)
    }

    // The seconds each batch of the run took and the steps its walkers were allowed.
    pub fn batches(&self) -> Vec<(f32, u64)> {
        self.read(|run| run.took.clone()).unwrap_or_default()
    }

    // Of the run: the most a unit ended past the slice of its frame, in seconds.
    pub fn overrun(&self) -> f32 {
        self.read(|run| run.overrun).unwrap_or(0.0)
    }

    // A picture is on its way and the card has not been heard of for longer than any unit or batch may take: a
    // completion that never came must not leave the screen waiting for ever.
    pub fn stalled(&self) -> bool {
        let silent = |heard: Instant| heard.elapsed().as_millis() > u128::from(SILENT);
        if let Some(pixels) = &self.pixels {
            return pixels.report.full.is_none() && silent(pixels.heard);
        }
        if self.base.is_some() {
            return false;
        }
        self.read(|run| run.full.is_none() && silent(run.heard))
            .unwrap_or(false)
    }

    fn read<T>(&self, of: impl FnOnce(&Run) -> T) -> Option<T> {
        let run = self.run.as_ref()?;
        Some(of(&run.lock().unwrap_or_else(|e| e.into_inner())))
    }

    fn parts(&self) -> Parts {
        Parts {
            device: self.device.clone(),
            queue: self.queue.clone(),
            failed: self.failed.clone(),
            pipelines: self.pipelines.clone(),
            particles: self.particles.clone(),
            longest: self.longest.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            waiter: self.waiter.clone(),
            #[cfg(test)]
            plain: self.plain,
        }
    }

    pub fn finished(&self) -> bool {
        let pixels = self.pixels.as_ref().is_some_and(|pixels| pixels.report.full.is_some());
        pixels || self.base.is_some() || self.read(|run| run.full.is_some()).unwrap_or(false)
    }

    // The picture is being drawn again with a margin around the view.
    pub fn widening(&self) -> bool {
        self.base.is_some() && self.run.is_some() && !self.widened()
    }

    // The picture drawn again with a margin around the view is whole.
    pub fn widened(&self) -> bool {
        self.base.is_some() && self.read(|run| run.full.is_some()).unwrap_or(false)
    }

    // The seconds the picture with a margin took, from when it was begun.
    pub fn widened_in(&self) -> Option<f32> {
        self.base.as_ref()?;
        self.read(|run| run.full).flatten()
    }

    // Draws the finished picture again behind the one on the screen, with so many cells of the scene beside the
    // view on every side. Nothing waits for it: the finished picture stays the run's until the wider one is whole,
    // and when the card has no room or no time for the wider one. False when nothing was begun.
    pub fn widen(&mut self, scene: &Scene, options: &RenderOptions, margin: [u32; 2], ctx: &egui::Context) -> bool {
        let whole = self.read(|run| run.full.is_some()).unwrap_or(false);
        if self.pixels.is_some() || self.base.is_some() || margin == [0; 2] || !whole {
            return false;
        }
        let times = self.times();
        let (Some(keys), Some((size, levels))) = (self.run_keys.take(), self.run_levels.take()) else {
            return false;
        };
        self.run = None;
        let begun = self.apart(|chaos| chaos.started(scene, options, margin, &keys, Instant::now(), None, ctx));
        if !begun {
            (self.run, self.run_levels, self.run_margin) = (None, None, [0; 2]);
        }
        self.run_keys = Some(keys);
        self.base = Some(Base { size, levels, times });
        begun
    }

    // What the card refuses of a picture with a margin costs the margin, not the card's part in the session.
    fn apart(&mut self, begin: impl FnOnce(&mut Self) -> bool) -> bool {
        let own = Arc::new(AtomicBool::new(false));
        let kept = std::mem::replace(&mut self.failed, own.clone());
        let begun = begin(self);
        self.failed = kept;
        begun && !own.load(Ordering::Relaxed)
    }

    // The levels of the view alone, under a picture that was drawn again with a margin: the view in the middle of
    // that picture is drawn from these, so it is the picture it was.
    pub fn inset(&self) -> Option<([u32; 2], Vec<&wgpu::TextureView>)> {
        let base = self.base.as_ref().filter(|_| self.widened())?;
        Some((base.size, base.levels.iter().collect()))
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
        if let Some(pixels) = &self.pixels {
            return (pixels.report.first, pixels.report.full);
        }
        if let Some(base) = &self.base {
            return base.times;
        }
        self.read(|run| (run.first, run.full)).unwrap_or((None, None))
    }

    pub fn levels(&self) -> Option<([u32; 2], Vec<&wgpu::TextureView>)> {
        if let Some(pixels) = self.pixels.as_ref().filter(|pixels| pixels.report.batches > 0) {
            return Some((pixels.size, pixels.levels.iter().collect()));
        }
        if let Some(base) = self.base.as_ref().filter(|_| !self.widened()) {
            return Some((base.size, base.levels.iter().collect()));
        }
        self.read(|run| run.batches > 0 && run.pictured())
            .filter(|shown| *shown)?;
        let (size, levels) = self.run_levels.as_ref()?;
        Some((*size, levels.iter().collect()))
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
        self.warmed >= WARM.len() && self.started(scene, options, [0; 2], keys, asked, limit, ctx)
    }

    // The cells beside the view, across and down, in the picture `levels` gives.
    pub fn margin(&self) -> [u32; 2] {
        if self.base.is_some() && !self.widened() {
            [0; 2]
        } else {
            self.run_margin
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn started(
        &mut self,
        scene: &Scene,
        options: &RenderOptions,
        margin: [u32; 2],
        keys: &[String],
        asked: Instant,
        limit: Option<f32>,
        ctx: &egui::Context,
    ) -> bool {
        if !self.works() {
            self.end();
            return false;
        }
        if self.keys() == Some(keys) {
            return true;
        }
        self.end();
        if self.refused.iter().any(|refused| refused == keys) {
            return false;
        }
        let pixelwise = scene.sheets.is_none() && !scene.layers.is_empty() && scene.layers.iter().all(field::drawn);
        if pixelwise || sheets::drawn(scene) {
            if options.width.max(options.height) > self.largest_side {
                return false;
            }
            let mut layers: Vec<Box<dyn Batched>> = Vec::new();
            let mut levels = Vec::new();
            if !pixelwise {
                let Some(layer) = self.fields.stacks.layer(scene, options, ctx) else {
                    return false;
                };
                levels.push(layer.levels().clone());
                layers.push(layer);
            }
            for fractal in scene.layers.iter().filter_map(|layer| layer.fractal()) {
                let Planned::Ready(layer) = self.fields.layer(fractal, options, scene.layers.len(), ctx) else {
                    return false;
                };
                levels.push(layer.levels().clone());
                layers.push(layer);
            }
            let (device, queue) = (&self.device, &self.queue);
            #[cfg(not(test))]
            self.longest.store(LONGEST, Ordering::Relaxed);
            self.pixels = Some(Pixels {
                keys: keys.to_vec(),
                heard: Instant::now(),
                size: [options.width, options.height],
                levels,
                chain: Chain::new(layers, device, queue, &self.failed, ctx, self.waiter(), asked),
                report: field::Report::default(),
            });
            return true;
        }
        let pipelines = self.pipelines.clone();
        let Some(pipelines) = pipelines.get() else {
            return false;
        };
        let inner = [options.width, options.height];
        let size = [inner[0] + 2 * margin[0], inner[1] + 2 * margin[1]];
        let cells = u64::from(size[0]) * u64::from(size[1]);
        let bounded = scene.layers.iter().any(|layer| layer.fractal().is_some());
        let Some((plans, walkers, weights)) = self.plans(scene, options, margin) else {
            return false;
        };
        #[cfg(not(test))]
        self.longest
            .store(if bounded { LONGEST } else { PATIENT }, Ordering::Relaxed);
        let goal = u64::from(options.samples_per_pixel) * u64::from(inner[0]) * u64::from(inner[1]);
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
                    let kernels = look(layer.tone(), size, inner);
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
                    let histogram = buffer(
                        "fractique histogram",
                        cells * 16,
                        storage | wgpu::BufferUsages::COPY_DST,
                    );
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
                        flame,
                        histogram,
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
                        checked: bounded || layer.camera().zoom <= WHOLE,
                        sparse: false,
                        landed: 0,
                        deep: None,
                        goal,
                        strip: None,
                    }
                })
                .collect::<Vec<_>>();
            let staging = buffer(
                "fractique landed",
                16 * layers.len() as u64,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            );
            Run {
                bounded,
                size,
                inner,
                seed: scene.seed,
                layers,
                walkers,
                steps: 1,
                sent: 0,
                fused: 0,
                batches: 0,
                staging,
                flight: Arc::default(),
                started: asked,
                submitted: asked,
                resolving: false,
                rows: 16,
                opened: asked,
                allowed: 0.0,
                told: Step::Waiting,
                over: None,
                stopped: false,
                overrun: 0.0,
                heard: Instant::now(),
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
        self.run_keys = Some(keys.to_vec());
        self.run_margin = margin;
        self.run_levels = Some((
            made.size,
            made.layers.iter().map(|layer| layer.levels.clone()).collect(),
        ));
        self.run = Some(Arc::new(Mutex::new(made)));
        true
    }

    // The layers as the shaders read them, the walkers and the layers' weights, or nothing for a scene or a view
    // the graphics card does not draw. Nothing is sent to the card.
    fn plans(
        &mut self,
        scene: &Scene,
        options: &RenderOptions,
        margin: [u32; 2],
    ) -> Option<(Vec<Plan>, u32, Vec<u64>)> {
        let inner = [options.width, options.height];
        let size = [inner[0] + 2 * margin[0], inner[1] + 2 * margin[1]];
        let cells = u64::from(size[0]) * u64::from(size[1]);
        let seen = u64::from(inner[0]) * u64::from(inner[1]);
        let layers = scene.layers.len() as u64;
        let bounded = scene.layers.iter().any(|layer| layer.fractal().is_some());
        if scene.sheets.is_some()
            || scene.layers.is_empty()
            || options.supersample != 1
            || size[0].max(size[1]) > self.largest_side
            || cells * 16
                > if margin == [0; 2] {
                    self.largest_buffer
                } else {
                    self.largest_buffer / 4
                }
            || cells * 24 * layers > if margin == [0; 2] { ROOM } else { ROOM / 2 }
            || bounded
                && scene
                    .layers
                    .iter()
                    .map(|layer| cells * taps(layer.tone(), inner))
                    .sum::<u64>()
                    > MOST_TAPS
        {
            return None;
        }
        let goal = u64::from(options.samples_per_pixel) * seen;
        let weights: Vec<u64> = scene
            .layers
            .iter()
            .map(|layer| layer.flame().map_or(1, weight))
            .collect();
        // Few enough that the unplotted steps of the first batch leave half of what it may take for points.
        let room = (MOST_STEPS * seen).min(MOST_IN_ALL) / layers;
        let heaviest = weights.iter().copied().max().unwrap_or(1);
        let fit = (room / (heaviest * 2 * u64::from(FUSE))).min(u64::from(MOST_WALKERS)) as u32;
        let walkers = if bounded {
            ((seen / 16) as u32)
                .min(fit)
                .div_ceil(GROUP)
                .clamp(1, MOST_WALKERS / GROUP)
                * GROUP
        } else {
            ((seen / 16) as u32).div_ceil(GROUP).clamp(GROUP, MOST_WALKERS / GROUP) * GROUP
        };
        let mut plans = Vec::new();
        for layer in &scene.layers {
            let plan = match (layer.flame(), layer.fractal()) {
                (Some(flame), _) => {
                    let finest = self.finest.unwrap_or(FINEST);
                    packed(flame, inner, margin, finest).map(|(data, plotted)| Plan::Flame(data, plotted))
                }
                // The walkers count the points they plot in 32 bits.
                (_, Some(shown)) if goal < 1 << 31 => self.recipe(shown).and_then(|recipe| {
                    if recipe.work(goal, walkers) > MOST_WORK * seen {
                        return None;
                    }
                    let (data, tables) = recipe.packed(shown, inner, margin, goal, walkers, self.finest)?;
                    Some(Plan::Orbit(data, tables))
                }),
                _ => None,
            };
            plans.push(plan?);
        }
        Some((plans, walkers, weights))
    }

    // Whether the card holds a picture of the scene with so many cells beside the view on every side.
    pub fn holds(&mut self, scene: &Scene, options: &RenderOptions, margin: [u32; 2]) -> bool {
        self.plans(scene, options, margin).is_some()
    }

    // Whether the graphics card draws this view of the scene, shaders that are still compiled aside. `deep` is a
    // view that holds little of a flame.
    pub fn draws(&mut self, scene: &Scene, options: &RenderOptions, deep: bool) -> bool {
        if !self.works() {
            return false;
        }
        if scene.sheets.is_none() && !scene.layers.is_empty() && scene.layers.iter().all(field::drawn) {
            let fits = options.width.max(options.height) <= self.largest_side;
            let mut fractals = scene.layers.iter().filter_map(|layer| layer.fractal());
            return fits && fractals.all(|fractal| self.fields.draws(fractal, options, scene.layers.len()));
        }
        let bounded = scene.layers.iter().any(|layer| layer.fractal().is_some());
        if deep && (bounded || matches!(self.particles.get(), Some(None))) {
            return false;
        }
        self.plans(scene, options, [0; 2]).is_some()
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
    fn refuse(&mut self, keys: Vec<String>) {
        self.refused.push(keys);
        if self.refused.len() > REFUSED {
            self.refused.remove(0);
        }
    }

    pub fn advance(&mut self, ctx: &egui::Context) -> Step {
        if let Some(pixels) = &mut self.pixels {
            // The picture before this one may have a batch on its way: this one waits for it, but not for ever.
            let waited = pixels.heard.elapsed().as_millis() < u128::from(2 * LONGEST);
            if waited && self.draining.as_ref().is_some_and(Chain::busy) {
                return Step::Waiting;
            }
            self.draining = None;
            pixels.chain.go();
            let fresh;
            (pixels.report, fresh) = pixels.chain.look();
            if fresh {
                pixels.heard = Instant::now();
            }
            if self.failed.load(Ordering::Relaxed) {
                self.pixels = None;
                return Step::Refused;
            }
            if pixels.report.late {
                if let Some(pixels) = self.pixels.take() {
                    self.refuse(pixels.keys);
                }
                return Step::Refused;
            }
            return if fresh { Step::Drawn } else { Step::Waiting };
        }
        let Some(shared) = self.run.clone() else {
            return Step::Waiting;
        };
        let parts = self.parts();
        let mut run = shared.lock().unwrap_or_else(|e| e.into_inner());
        // The frame opens a slice of the card's time: units follow one another in it.
        (run.opened, run.allowed) = (Instant::now(), pace::slice());
        let step = parts.pump(&shared, &mut run, ctx);
        let step = step.max(std::mem::replace(&mut run.told, Step::Waiting));
        let over = run.over;
        drop(run);
        // The picture on the screen is not the one this run draws: nothing of it is shown before it is whole.
        let step = if self.base.is_some() && over.is_none() {
            Step::Waiting
        } else {
            step
        };
        if let Some(refuse) = over {
            self.run = None;
            self.run_levels = None;
            // A picture with a margin that is given up leaves the picture it was to widen.
            if self.base.is_some() {
                self.run_margin = [0; 2];
                return Step::Waiting;
            }
            if let Some(keys) = self.run_keys.take().filter(|_| refuse) {
                self.refuse(keys);
            }
            return Step::Refused;
        }
        step
    }
}

impl Parts {
    // Takes in the unit that has finished and sends the next one, if the frame's slice has room for it. After
    // Drawn the levels are new and the picture is to be drawn from them again.
    fn pump(&self, shared: &Arc<Mutex<Run>>, run: &mut Run, ctx: &egui::Context) -> Step {
        if run.stopped || run.over.is_some() {
            return Step::Waiting;
        }
        if self.failed.load(Ordering::Relaxed) {
            run.over = Some(false);
            return Step::Refused;
        }
        // A view that holds little of a flame is drawn from particles, where the card has the shaders for it and
        // the scene is of flames alone.
        let shallow = run.bounded || matches!(self.particles.get(), Some(None));
        {
            let mut flight = run.flight.lock().unwrap_or_else(|e| e.into_inner());
            if flight.busy {
                return Step::Waiting;
            }
            if let Some(at) = flight.finished.take() {
                run.heard = at;
                let mut sparse = false;
                let mut told = flight.landed.as_slice();
                // A submission that drew levels walked nothing: what it tells was told before.
                let walked = if run.resolving { 0 } else { usize::MAX };
                for layer in run.layers.iter_mut().take(walked) {
                    let Some((counts, rest)) = told.split_at_checked(layer.told_in()) else {
                        break;
                    };
                    told = rest;
                    if let Some(plan) = &mut layer.deep {
                        match plan.stage {
                            Stage::Pilot => plan.piloted(counts),
                            Stage::Window => plan.swept(counts),
                            Stage::Plot => {
                                layer.landed = u64::from(counts[1]);
                            }
                        }
                    } else if layer.orbit {
                        layer.done = u64::from(counts[1]);
                    } else {
                        layer.landed = u64::from(counts[0]);
                        // The unit of unplotted steps alone tells nothing of where the points land.
                        if layer.done == 0 {
                            continue;
                        }
                        layer.sparse = (layer.landed as f64) < deep::IN_VIEW * layer.plotted * layer.done as f64;
                        #[cfg(test)]
                        if self.plain.is_some() {
                            layer.sparse = false;
                        }
                        layer.checked |= !layer.sparse;
                        sparse |= layer.sparse && shallow;
                    }
                }
                // A card that would take longer than the processor is told to stop: the processor renders. Its
                // own work counts, not the wait for its shaders or for a frame. A point of a deep view costs
                // the processor more steps too.
                let since = run.started.elapsed().as_secs_f32();
                let took = at.duration_since(run.submitted).as_secs_f32().max(1e-4);
                // A unit of a deep view tells of every particle and is dearer to send: it is a longer one.
                let unit = if run.layers.iter().any(|layer| layer.deep.is_some()) {
                    DEEP_UNIT
                } else {
                    UNIT
                };
                let worked = run.spent + took;
                let share = |layer: &Layer| layer.done as f32 / layer.goal.max(1) as f32;
                let least = run.layers.iter().map(share).fold(f32::INFINITY, f32::min);
                let plans = run.layers.iter().filter_map(|layer| layer.deep.as_ref());
                let cost = plans.map(deep::Plan::cost).max().unwrap_or(1) as f32;
                let slow = least < 1.0
                    && run.limit.is_some_and(|limit| {
                        let left = if least <= 0.0 { 1.0 } else { 1.0 / least };
                        // The card works a share of the time only: that is how long the picture takes.
                        worked * left > limit * cost * pace::SHARE
                    });
                let idle = least <= 0.0 && worked >= IDLE;
                let late = took * 1000.0 > self.longest.load(Ordering::Relaxed) as f32;
                if late {
                    log::warn!("the graphics card took {took:.2} s for one batch: the picture is given up");
                }
                if flight.broken || sparse || slow || idle || late {
                    if flight.broken {
                        self.failed.store(true, Ordering::Relaxed);
                    }
                    drop(flight);
                    run.over = Some(true);
                    return Step::Refused;
                }
                if run.pictured() {
                    run.first.get_or_insert(since);
                }
                run.spent += took;
                run.took.push((took, run.working));
                let past = at.duration_since(run.opened).as_secs_f32() - run.allowed;
                run.overrun = run.overrun.max(past);
                // The submission that looks the tiles over says nothing of how long a strip takes.
                let stripped = run
                    .layers
                    .iter()
                    .any(|layer| matches!(layer.strip, Some((Some(row), ..)) if row > 0));
                if run.resolving && (stripped || run.layers.iter().all(|layer| layer.strip.is_none())) {
                    // Rows where the picture is empty cost little and say little of the next: twice as many at most,
                    // four times while a strip takes less than a quarter of a unit.
                    let most = if took < unit / 4.0 { 4.0 } else { 2.0 };
                    let rows = pace::next(f64::from(run.rows), took, unit).min(most * f64::from(run.rows));
                    let rows = rows.ceil() as u32;
                    run.rows = rows.clamp(8, run.size[1].max(8));
                }
                if run.sent > 0 || run.fused > 0 {
                    run.steps = following(run.sent, run.fused, took, unit);
                }
                if run.layers.iter().all(Layer::settled) {
                    run.full.get_or_insert(since);
                    run.layers.iter_mut().for_each(|layer| layer.work = None);
                }
            }
        }
        if run.full.is_some() {
            return Step::Waiting;
        }
        // A unit begins only inside the frame's slice: the next frame opens another.
        if !pace::within(run.opened.elapsed().as_secs_f32(), pace::slice().min(run.allowed)) {
            return Step::Waiting;
        }
        // wgpu refuses to configure the window's surface while another thread submits: the next frame goes on.
        let _gate = match eframe::egui_wgpu::SURFACE_GATE.try_read() {
            Ok(gate) => gate,
            Err(std::sync::TryLockError::Poisoned(gate)) => gate.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return Step::Waiting,
        };
        let Some(pipelines) = self.pipelines.get() else {
            return Step::Waiting;
        };
        // The cells of the view, and how many times as many, and as wide, the picture with its margin is.
        let cells = f64::from(run.inner[0]) * f64::from(run.inner[1]);
        let spread = f64::from(run.size[0]) * f64::from(run.size[1]) / cells;
        let wide =
            (f64::from(run.size[0]) / f64::from(run.inner[0])).min(f64::from(run.size[1]) / f64::from(run.inner[1]));
        let (device, failed) = (self.device.clone(), self.failed.clone());
        let deeper = run.layers.iter().any(|layer| layer.sparse && layer.deep.is_none());
        let particles = match self.particles.get() {
            Some(particles) => particles.as_ref(),
            None if deeper => return Step::Waiting,
            None => None,
        };
        if deeper {
            if particles.is_none() {
                run.over = Some(true);
                return Step::Refused;
            }
            guarded(&device, &failed, ctx, || {
                let buffer = |label, size: u64, usage| {
                    self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(label),
                        size,
                        usage,
                        mapped_at_creation: false,
                    })
                };
                let (storage, moved) = (
                    wgpu::BufferUsages::STORAGE,
                    wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                );
                for layer in run
                    .layers
                    .iter_mut()
                    .filter(|layer| layer.sparse && layer.deep.is_none())
                {
                    let Some(work) = &mut layer.work else {
                        continue;
                    };
                    let count = deep::particles((cells * spread) as u64, GROUP);
                    let slots = buffer("fractique particles", u64::from(count) * deep::SLOT * 4, storage);
                    let notes = buffer(
                        "fractique notes",
                        (deep::HEAD as u64 + 2 * u64::from(count)) * 4,
                        storage | moved,
                    );
                    work.batch = buffer(
                        "fractique batch",
                        size_of::<deep::Batch>() as u64,
                        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    );
                    work.walking = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("fractique particles"),
                        layout: &pipelines.walking,
                        entries: &[
                            bound(0, &work.flame),
                            bound(1, &work.batch),
                            bound(2, &slots),
                            bound(3, &work.histogram),
                            bound(4, &notes),
                        ],
                    });
                    work.landed = notes;
                    layer.deep = Some(deep::Plan::new(layer.zoom, layer.plotted, count, wide));
                    (layer.done, layer.resolved) = (0, 0);
                    layer.landed = 0;
                }
                let told: usize = run.layers.iter().map(Layer::told_in).sum();
                run.staging = buffer(
                    "fractique landed",
                    4 * told as u64,
                    wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                );
            });
            if failed.load(Ordering::Relaxed) {
                run.over = Some(false);
                return Step::Refused;
            }
            // The steps were sized by the plain chaos game: a point of a deep view is not a step of it.
            run.steps = (run.steps / 4).max(1);
        }
        // Before the submission: a graphics card that is the processor in disguise works inside it.
        run.submitted = Instant::now();
        let (mut sent, mut fused, mut working, mut resolving, mut beginning) = (0, 0, 0, false, false);
        let index = guarded(&device, &failed, ctx, || {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fractique chaos"),
            });
            let (walkers, count) = (run.walkers, run.layers.len() as u64);
            let bounded = run.bounded;
            let shorter = run.size[0].min(run.size[1]);
            // They are drawn a strip of rows a submission, each within the slice, and nothing is walked meanwhile:
            // the strips are of one histogram.
            let stripping = run.layers.iter().any(|layer| layer.strip.is_some());
            let rows = run.rows;
            resolving = stripping;
            let mut told = 0;
            for (index, layer) in run.layers.iter_mut().enumerate() {
                let (from, telling) = (told, 4 * layer.told_in() as u64);
                told += telling;
                let Some(work) = &layer.work else {
                    continue;
                };
                // The levels are drawn twice: for the first picture and for the last. Drawing them costs the same
                // after every batch and nothing shows them in between.
                let goal = layer.goal;
                let first = (u64::from(FIRST_SAMPLES) * cells as u64).min(goal);
                if stripping {
                    if let Some((row, scale, done)) = layer.strip {
                        // A deep view is as bright as its exposure says over the points that landed in the view,
                        // which the card has told of by now.
                        let scale = match &layer.deep {
                            Some(plan) if row.is_none() => plan.exposure * cells / layer.landed.max(1) as f64,
                            _ => scale,
                        };
                        let look = &layer.kernels;
                        let from_row = row.unwrap_or(0);
                        let tall = rows.min(look.size[1] - from_row).div_ceil(8);
                        self.queue.write_buffer(
                            &work.look,
                            16,
                            bytemuck::bytes_of(&[scale as f32, from_row as f32, 0.0, 0.0]),
                        );
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("fractique levels"),
                            timestamp_writes: None,
                        });
                        pass.set_bind_group(0, &work.looking, &[]);
                        if row.is_none() {
                            pass.set_pipeline(&pipelines.survey);
                            pass.dispatch_workgroups(look.size[2].div_ceil(8), look.size[3].div_ceil(8), 1);
                            layer.strip = Some((Some(0), scale, done));
                        } else {
                            pass.set_pipeline(&pipelines.resolve);
                            pass.dispatch_workgroups(look.size[0].div_ceil(8), tall, 1);
                            let next = from_row + tall * 8;
                            if next >= look.size[1] {
                                (layer.resolved, layer.strip) = (done, None);
                            } else {
                                layer.strip = Some((Some(next), scale, done));
                            }
                        }
                    }
                    encoder.copy_buffer_to_buffer(&work.landed, 0, &run.staging, from, telling);
                    continue;
                }
                let plain = |points: u64| layer.zoom * layer.zoom * cells / points as f64;
                #[cfg(test)]
                let before = layer.done.max(1);
                #[cfg(test)]
                let plain = |points: u64| match self.plain {
                    Some(exposure) => exposure * cells / (layer.landed.max(1) as f64 * points as f64 / before as f64),
                    None => plain(points),
                };
                let left = goal.saturating_sub(layer.done);
                if let (Some(plan), Some(particles)) = (&layer.deep, particles) {
                    let few = u64::from(plan.particles);
                    let cost = plan.cost();
                    let steps = if plan.stage == Stage::Plot && plan.exposure > 0.0 && left > 0 {
                        let points = u64::from(run.steps) * u64::from(walkers) / cost;
                        points
                            .min(MOST_DEEP / (cost * layer.weight))
                            .min(left)
                            .div_ceil(few)
                            .max(1)
                    } else {
                        0
                    };
                    let batch = plan.batch(run.seed, index as u32, shorter, steps as u32);
                    self.queue.write_buffer(&work.batch, 0, bytemuck::bytes_of(&batch));
                    if plan.stage != Stage::Plot {
                        self.queue
                            .write_buffer(&work.landed, 0, bytemuck::cast_slice(&[0u32; deep::HEAD]));
                    }
                    if plan.stage == Stage::Pilot {
                        encoder.clear_buffer(&work.histogram, 0, None);
                    }
                    if plan.stage == Stage::Window {
                        let after = 4 * (deep::HEAD as u64 + few);
                        self.queue
                            .write_buffer(&work.landed, after, bytemuck::cast_slice(&plan.sources()));
                    }
                    if plan.stage != Stage::Plot || steps > 0 {
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("fractique particles"),
                            timestamp_writes: None,
                        });
                        pass.set_bind_group(0, &work.walking, &[]);
                        let groups = plan.particles / GROUP;
                        match plan.stage {
                            Stage::Pilot => {
                                pass.set_pipeline(&particles.pilot);
                                pass.dispatch_workgroups(groups, 1, 1);
                                working += few * u64::from(deep::MEMORY + batch.steps) * layer.weight;
                            }
                            Stage::Window => {
                                pass.set_pipeline(&particles.refill);
                                pass.dispatch_workgroups(groups, 1, 1);
                                pass.set_pipeline(&particles.sweep);
                                for _ in 0..deep::SWEEPS {
                                    pass.dispatch_workgroups(groups, 1, 1);
                                }
                                if plan.more() {
                                    pass.set_pipeline(&particles.mark);
                                    pass.dispatch_workgroups(groups, 1, 1);
                                }
                                working += few * u64::from(deep::SWEEPS) * plan.sweep() * layer.weight;
                            }
                            Stage::Plot => {
                                pass.set_pipeline(&particles.sweep);
                                pass.dispatch_workgroups(groups, 1, 1);
                                working += steps * few * cost * layer.weight;
                                sent = sent.max((steps * few * cost).div_ceil(u64::from(walkers)) as u32);
                            }
                        }
                    }
                    if plan.stage == Stage::Plot {
                        // A view the flame has nothing in is a picture too.
                        layer.done = if plan.exposure > 0.0 {
                            layer.done + steps * few
                        } else {
                            goal
                        };
                        let due = layer.resolved == 0 || layer.done >= goal;
                        if layer.done >= first && layer.done > layer.resolved && due {
                            layer.strip = Some((None, 0.0, layer.done));
                            beginning = true;
                        }
                    }
                    encoder.copy_buffer_to_buffer(&work.landed, 0, &run.staging, from, telling);
                    continue;
                }
                let due = (layer.resolved == 0 && layer.done >= first) || layer.done >= goal;
                if layer.orbit && layer.done > layer.resolved && due {
                    layer.strip = Some((None, plain(layer.done), layer.done));
                    beginning = true;
                }
                if left == 0 {
                    encoder.copy_buffer_to_buffer(&work.landed, 0, &run.staging, from, telling);
                    continue;
                }
                let few = u64::from(walkers);
                // A flame's walkers take half of their unplotted steps in a unit of their own and the rest with the
                // first they plot: together they are 25 steps of every walker, 8 ms for a dear flame.
                let fuse = match (layer.orbit, run.batches) {
                    (false, 0) => FUSE / 2,
                    (false, 1) => FUSE - FUSE / 2,
                    _ => 0,
                };
                fused = fused.max(fuse);
                let room = (MOST_STEPS * cells as u64).min(MOST_IN_ALL) / (count * few * layer.weight);
                let most = if bounded {
                    room.saturating_sub(u64::from(fuse)).max(1)
                } else {
                    u64::MAX
                };
                let own = if run.batches == 1 {
                    run.steps.saturating_sub(fuse / 3).max(1)
                } else {
                    run.steps
                };
                let steps = if run.batches == 0 && !layer.orbit {
                    0
                } else {
                    (u64::from(own) * u64::from(walkers)).min(left).div_ceil(few).min(most)
                };
                let budget = if layer.orbit { (steps * UNSEEN).min(most) } else { steps };
                if !layer.orbit {
                    layer.done += steps * few;
                }
                let batch = Batch {
                    steps: steps as u32,
                    fuse: match (layer.orbit, run.batches) {
                        (true, 0) => FUSE,
                        (false, 0) => fuse,
                        _ => 0,
                    },
                    seed: [run.seed as u32, (run.seed >> 32) as u32],
                    layer: index as u32,
                    walkers: few as u32,
                    budget: budget as u32,
                    idle: if run.batches == 1 { fuse } else { 0 },
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
                if !layer.orbit && ((layer.resolved == 0 && layer.done >= first) || layer.done >= goal) {
                    layer.strip = Some((None, plain(layer.done), layer.done));
                    beginning = true;
                }
                sent = sent.max((steps * few).div_ceil(u64::from(walkers)) as u32);
                working += (budget + u64::from(fuse)) * few * layer.weight;
                encoder.copy_buffer_to_buffer(&work.landed, 0, &run.staging, from, telling);
            }
            self.queue.submit([encoder.finish()])
        });
        run.sent = sent;
        run.fused = fused;
        run.working = working;
        run.resolving = resolving;
        if beginning {
            // The rows of a picture with more samples cost more: each drawing of the levels begins with few.
            run.rows = 16;
        }
        run.batches += 1;
        run.flight.lock().unwrap_or_else(|e| e.into_inner()).busy = true;
        let (flight, staging, ctx) = (run.flight.clone(), run.staging.clone(), ctx.clone());
        let (parts, shared) = (self.clone(), shared.clone());
        run.staging.slice(..).map_async(wgpu::MapMode::Read, move |mapped| {
            {
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
            }
            // The end of a unit sends the next, while the frame's slice lasts: nothing waits for a frame of the
            // window. Where the run is held, by the frame or by this very thread within a submission, the frame
            // goes on with it.
            if let Ok(mut run) = shared.try_lock() {
                let step = parts.pump(&shared, &mut run, &ctx);
                run.told = run.told.max(step);
            }
            ctx.request_repaint();
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.waiter.send(index);
        #[cfg(target_arch = "wasm32")]
        let _ = index;
        let pictured = run.pictured();
        if self.failed.load(Ordering::Relaxed) {
            run.over = Some(false);
            Step::Refused
        } else if pictured {
            Step::Drawn
        } else {
            Step::Waiting
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    pub(crate) struct Card {
        device: wgpu::Device,
        queue: wgpu::Queue,
        ctx: egui::Context,
        pub(crate) chaos: Chaos,
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

    pub(crate) fn card() -> Option<Card> {
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
        let mut chaos = Chaos::new(&adapter, &device, &queue, &ctx)?;
        chaos.warmed = WARM.len();
        let preview = Preview::new(&adapter, &device, &queue, &ctx)?;
        while chaos.pending() || chaos.works() && chaos.particles.get().is_none() {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        eprintln!("graphics: {:?}", adapter.get_info());
        // A software adapter compiles a shader when it is first run.
        if adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            chaos.longest.store(PATIENT, Ordering::Relaxed);
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
        pub(crate) fn drawn(&mut self, scene: &Scene, options: &RenderOptions) -> Option<(Vec<u8>, f32, f32)> {
            self.drawn_wide(scene, options, [0; 2])
        }

        // The same of a picture drawn with a margin around the view: the view alone is read back.
        pub(crate) fn drawn_wide(
            &mut self,
            scene: &Scene,
            options: &RenderOptions,
            margin: [u32; 2],
        ) -> Option<(Vec<u8>, f32, f32)> {
            let keys = vec![format!("{options:?} {margin:?} {}", scene.to_json())];
            self.chaos.end();
            let now = Instant::now();
            while !self.chaos.started(scene, options, margin, &keys, now, None, &self.ctx) {
                if !self.chaos.pending() {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
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
            let (whole, levels) = self.chaos.levels()?;
            let size = [options.width, options.height];
            let margin = self.chaos.margin();
            if !self.preview.draw_levels(scene, whole, size, &levels, &self.ctx) {
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
                    origin: wgpu::Origin3d {
                        x: margin[0],
                        y: margin[1],
                        z: 0,
                    },
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
            Some((rgba, first, full))
        }

        // The view of a picture that is drawn alone first and again with a margin behind it, and the mean light
        // of what is beside the view.
        pub(crate) fn widened(
            &mut self,
            scene: &Scene,
            options: &RenderOptions,
            margin: [u32; 2],
        ) -> Option<(Vec<u8>, f64)> {
            self.drawn(scene, options)?;
            if !self.chaos.widen(scene, options, margin, &self.ctx) {
                return None;
            }
            while !self.chaos.widened() {
                if self.chaos.advance(&self.ctx) == Step::Refused || self.chaos.read(|_| ()).is_none() {
                    return None;
                }
                self.wait();
            }
            let (whole, levels) = self.chaos.levels()?;
            let (inner, inset) = self.chaos.inset()?;
            if !self
                .preview
                .draw_wide(scene, whole, inner, &levels, Some(&inset), &self.ctx)
            {
                return None;
            }
            let read = |card: &Self, margin: [u32; 2]| {
                let (sender, receiver) = std::sync::mpsc::channel();
                card.preview.read(false, margin, &card.ctx, move |pixels| {
                    let _ = sender.send(pixels);
                })?;
                card.wait();
                receiver.recv().ok()?
            };
            let (all, size) = read(self, [0; 2])?;
            // What an export of the picture on the screen reads back: the view, without the margin.
            let (cut, cut_size) = read(self, margin)?;
            assert_eq!(cut_size, inner);
            let (mut view, mut beside, mut count) = (Vec::new(), 0.0, 0.0);
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let at = ((y * size[0] + x) * 4) as usize;
                    let inside =
                        x >= margin[0] && x < margin[0] + inner[0] && y >= margin[1] && y < margin[1] + inner[1];
                    if inside {
                        view.extend(&all[at..at + 4]);
                    } else {
                        beside += all[at..at + 3].iter().map(|c| f64::from(*c)).sum::<f64>();
                        count += 3.0;
                    }
                }
            }
            assert!(cut == view, "the picture read back without its margin is the view");
            Some((view, beside / count))
        }
    }

    const BLOCK: usize = 8;

    // How far two pictures are apart, on the scale of 0 to 255: the mean and the largest difference between the
    // averages of their 8×8 blocks, so that the noise of single pixels does not count.
    pub(crate) fn apart(one: &[u8], other: &[u8], width: usize, height: usize) -> (f64, f64) {
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
    fn a_picture_cut_at_any_stage_does_not_keep_the_next_from_coming() {
        let Some(mut card) = card() else {
            return;
        };
        let options = options(320, 200);
        let flame = presets::get("solar-mandala").expect("the preset exists");
        let other = presets::get("aurora-spiral").expect("the preset exists");
        let ship = fractal::preset("burning_ship-whole").expect("the preset exists");
        let seahorse = fractal::preset("mandelbrot-seahorse-valley").expect("the preset exists");
        let clifford = fractal::preset("clifford-classic").expect("the preset exists");
        // Every pair of paths: the chaos game, an orbit, a kind drawn pixel by pixel.
        let scenes = [&flame, &clifford, &ship];
        for (first, cut) in scenes.iter().enumerate() {
            for after in [&other, &seahorse, &clifford] {
                // Cut before anything is sent, after the first unit, and a few units on, with one on its way.
                for units in [0, 1, 2, 5, 9] {
                    let keys = vec![format!("cut {first} {units}")];
                    card.chaos.end();
                    while !card.chaos.begin(cut, &options, &keys, Instant::now(), None, &card.ctx) {
                        assert!(card.chaos.pending(), "the card draws the scene");
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    for unit in 0..units {
                        card.chaos.advance(&card.ctx);
                        // The last unit is left on its way: the next picture begins while it is.
                        if unit + 1 < units {
                            card.wait();
                        }
                    }
                    assert!(card.drawn(after, &options).is_some(), "after {first} cut at {units}");
                    assert!(!card.chaos.stalled());
                }
            }
        }
    }

    #[test]
    fn the_first_batches_of_a_flame_fit_the_slice_whatever_the_flame_before_cost() {
        let slice = pace::slice_of(false);
        let walkers = 262_144.0;
        // Seconds a step: a cheap flame, and one twenty times dearer that is drawn right after it.
        // An unplotted step costs a third of a plotted one, or as much.
        for (each, unplotted) in [(6.0e-11, 1.0 / 3.0), (1.2e-9, 1.0 / 3.0), (6.0e-11, 1.0), (1.2e-9, 1.0)] {
            let seconds =
                |sent: u32, fused: u32| ((f64::from(sent) + unplotted * f64::from(fused)) * walkers * each) as f32;
            // The first unit is half of the unplotted steps and nothing else, as small as a unit can be.
            let (mut sent, mut fused) = (0, FUSE / 2);
            let mut took = seconds(sent, fused);
            assert!(
                took <= pace::slice_of(true) * 1.05,
                "{each}: the first unit takes {took} s"
            );
            let mut filled = 0;
            for batch in 1..8 {
                let steps = following(sent, fused, took, slice);
                // The second takes the other half of the unplotted steps with its first plotted ones.
                (sent, fused) = if batch == 1 {
                    (steps.saturating_sub(FUSE / 2 / 3).max(1), FUSE - FUSE / 2)
                } else {
                    (steps, 0)
                };
                took = seconds(sent, fused);
                assert!(
                    took <= slice * 1.001,
                    "{each} {unplotted}: batch {batch} takes {took} s"
                );
                filled += u32::from(took >= 0.6 * slice);
            }
            assert!(filled >= 4, "{each} {unplotted}: {filled}");
        }
        // A batch that overran is cut to the slice at once.
        assert_eq!(following(100, 0, 2.0 * slice, slice), 50);
        assert_eq!(following(1, 0, 9.0 * slice, slice), 1);
    }

    #[test]
    fn flames_are_packed_as_the_shader_reads_them() {
        assert_eq!(size_of::<Packed>(), 80 + 65 * 80 + 256 * 32 + 1024 + 512);
        let scene = presets::get("solar-mandala").expect("the preset exists");
        let flame = scene.layers[0].flame().expect("the preset is a flame");
        let (data, plotted) =
            packed(flame, [640, 360], [0; 2], FINEST).expect("the flame is one the graphics card draws");
        assert_eq!(data.counts[0] as usize, flame.transforms.len());
        assert_eq!(data.counts[1], flame.symmetry.unsigned_abs());
        assert!(plotted > 0.0 && plotted <= 1.0);
        let weights: f64 = flame.transforms.iter().map(|transform| transform.weight).sum();
        assert_eq!(data.xfs[flame.transforms.len() - 1].after[3], weights as f32);

        let mut deep = flame.clone();
        deep.camera.zoom = 1.0e7;
        assert!(packed(&deep, [640, 360], [0; 2], FINEST).is_none());
    }

    #[test]
    fn the_kernels_are_those_of_the_density_estimation() {
        let scene = presets::get("aurora-spiral").expect("the preset exists");
        let look = look(scene.layers[0].tone(), [3840, 2160], [3840, 2160]);
        assert_eq!(look.widths[0][1], 17);
        assert_eq!(look.widths[8][0], 4);
        assert_eq!(look.widths[1023][3], 0);
        assert_eq!(look.kernels[17][0], 9.0);
        assert_eq!(look.kernels[0], [0.0, (1.0 / 0.18f64) as f32, 1.0, 0.0]);
        assert_eq!(look.size[2..], [240, 135]);
        // A picture with a margin has the kernels of its view, over more cells.
        let wide = super::look(scene.layers[0].tone(), [1920, 1080], [1280, 720]);
        let plain = super::look(scene.layers[0].tone(), [1280, 720], [1280, 720]);
        assert_eq!(wide.kernels, plain.kernels);
        assert_eq!(wide.widths, plain.widths);
        assert_eq!(wide.size, [1920, 1080, 120, 68]);
    }

    // The view in the middle of a picture drawn with a margin is the picture drawn without one: the same samples
    // a pixel, the same kernels, the same brightness. Drawn behind a finished picture, as the app draws it, the
    // view stays the picture it was, pixel for pixel, for a deep view too.
    #[test]
    fn the_view_of_a_picture_with_a_margin_is_the_picture_without_one() {
        let Some(mut card) = card() else {
            eprintln!("no graphics card with compute shaders: nothing is compared");
            return;
        };
        let options = options(320, 200);
        let preset = |name: &str| {
            presets::get(name)
                .or_else(|| fractal::preset(name))
                .expect("the preset exists")
        };
        let scenes = [
            ("aurora-spiral", preset("aurora-spiral"), false),
            ("solar-mandala", preset("solar-mandala"), false),
            ("clifford-classic", preset("clifford-classic"), false),
            ("lorenz-butterfly", preset("lorenz-butterfly"), false),
            ("aurora-spiral × 2", magnified("aurora-spiral", 2.0), true),
            ("aurora-spiral × 14", magnified("aurora-spiral", 14.0), true),
        ];
        println!("| Scene | Margin | Alone: mean, most | Drawn behind: mean, most | Light beside the view |");
        println!("| --- | --- | --- | --- | --- |");
        for (name, scene, deep) in &scenes {
            let (plain, _, _) = card
                .drawn(scene, &options)
                .unwrap_or_else(|| panic!("the card draws {name}"));
            for margin in [[80, 50], [160, 100]] {
                // The picture with a margin alone: its view differs by what the kernels at the view's edge find in
                // the margin and by the glow that comes in from it. A deep view plots the points of its view into
                // the whole picture, and is another draw of the particles: it is not held to the view.
                let (wide, _, _) = card
                    .drawn_wide(scene, &options, margin)
                    .unwrap_or_else(|| panic!("the card draws {name} with a margin"));
                let (mean, largest) = apart(&wide, &plain, 320, 200);
                assert!(
                    *deep || mean < 0.2 && largest < 12.0,
                    "{name} with {margin:?}: {mean:.2}, {largest:.1}"
                );

                let (kept, beside) = card
                    .widened(scene, &options, margin)
                    .unwrap_or_else(|| panic!("the card draws {name} again with a margin"));
                assert_eq!(card.chaos.margin(), margin);
                // Inside the band at its edge, where it fades into the picture with the margin, the view is the
                // picture it was.
                let band = crate::preview::feather([320, 200]).ceil() as usize + 1;
                let inside = |rgba: &[u8]| -> Vec<u8> {
                    (band..200 - band)
                        .flat_map(|y| rgba[(y * 320 + band) * 4..(y * 320 + 320 - band) * 4].iter().copied())
                        .collect()
                };
                let (same, most) = apart(&kept, &plain, 320, 200);
                assert!(
                    inside(&kept) == inside(&plain),
                    "{name} with {margin:?}: the view changed"
                );
                assert!(
                    *deep || same < 0.2 && most < 12.0,
                    "{name} with {margin:?}: {same:.2}, {most:.1} at its edge"
                );
                println!(
                    "| {name} | {} | {mean:.3}, {largest:.2} | {same:.3}, {most:.2} | {beside:.1} |",
                    margin[0]
                );
                assert!(beside > 1.0, "{name} with {margin:?}: nothing is drawn beside the view");
            }
        }
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

    // The preset magnified so many times about a point of its picture that is lit as half of the lit ones are.
    fn magnified(name: &str, by: f64) -> Scene {
        let mut scene = presets::get(name).expect("the preset exists");
        let whole = RenderOptions {
            samples_per_pixel: 8,
            ..options(320, 200)
        };
        let picture = fractique_core::render(&scene, &whole, &Control::default()).expect("it renders");
        let light = |at: &usize| {
            picture.rgba[at * 4..at * 4 + 3]
                .iter()
                .map(|c| u32::from(*c))
                .sum::<u32>()
        };
        let mut lit: Vec<usize> = (0..320 * 200).filter(|at| light(at) > 96).collect();
        lit.sort_by_key(light);
        let middle = lit[lit.len() / 2];
        let (right, down) = ((middle % 320) as f64 + 0.5 - 160.0, (middle / 320) as f64 + 0.5 - 100.0);
        let camera = scene.layers[0].camera_mut();
        *camera = camera.looking_at(right / 200.0, down / 200.0, by);
        scene
    }

    #[test]
    fn a_view_with_little_of_the_flame_is_drawn_from_particles() {
        let Some(mut card) = card() else {
            return;
        };
        let options = options(320, 200);
        for by in [4.0, 40.0] {
            let scene = magnified("aurora-spiral", by);
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let (quick, _, _) = card
                .drawn(&scene, &options)
                .expect("the graphics card draws a deep view");
            let deep = card.chaos.read(|run| run.layers[0].deep.is_some());
            assert!(
                deep.expect("the run is kept"),
                "{by} times: the view is not drawn from particles"
            );
            // A deep view is as far from itself with other random numbers: the processor's tells how far.
            let mut other = scene.clone();
            other.seed += 1;
            let again = fractique_core::render(&other, &options, &Control::default()).expect("it renders");
            let (grain, _) = apart(&again.rgba, &exact.rgba, 320, 200);
            let (mean, _) = apart(&quick, &exact.rgba, 320, 200);
            assert!(
                mean < 2.0 * grain + 1.0,
                "{by} times: {mean:.2} on average, the processor's own {grain:.2}"
            );
        }
        let request = fractique_core::wallpaper::Request {
            layout: None,
            material: None,
            ratio: 0.625,
            dark: true,
            hue: None,
        };
        let mut wallpaper = fractique_core::wallpaper::random_wallpaper(3, &request);
        wallpaper.layers = presets::get("aurora-spiral").expect("the preset exists").layers;
        assert!(card.drawn(&wallpaper, &options).is_none());
    }

    // FRACTIQUE_COMPARE=folder cargo test --release -p fractique-app --lib deep_views -- --ignored --nocapture
    // writes the table of differences and times for three presets magnified 3 to 1000 times, and for each the
    // picture of the graphics card above the processor's. FRACTIQUE_COMPARE_SIZE, _SAMPLES and _ONLY as below.
    #[test]
    #[ignore = "a report, not a check"]
    fn deep_views_on_the_graphics_card_beside_the_processor() {
        let mut card = card().expect("a graphics card with compute shaders");
        let samples = std::env::var("FRACTIQUE_COMPARE_SAMPLES")
            .ok()
            .and_then(|samples| samples.parse().ok())
            .unwrap_or(32);
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let size: Vec<u32> = std::env::var("FRACTIQUE_COMPARE_SIZE")
            .unwrap_or_else(|_| "640x360".to_owned())
            .split('x')
            .filter_map(|side| side.parse().ok())
            .collect();
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        let options = RenderOptions {
            samples_per_pixel: samples,
            ..options(size[0], size[1])
        };
        println!("| Scene | Mean | Largest | Another seed | First picture, s | All samples, s | Processor, s |");
        println!("| --- | --- | --- | --- | --- | --- | --- |");
        for preset in ["aurora-spiral", "solar-mandala", "ember-vortex"] {
            for by in [3.0, 14.0, 100.0, 1000.0] {
                let name = format!("{preset} × {by}");
                if only.as_ref().is_some_and(|only| !name.contains(only.as_str())) {
                    continue;
                }
                let scene = magnified(preset, by);
                let started = Instant::now();
                let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
                let slow = started.elapsed().as_secs_f32();
                let Some((quick, first, full)) = card.drawn(&scene, &options) else {
                    println!("| {name} | not drawn | | | | | {slow:.2} |");
                    continue;
                };
                let (mean, largest) = apart(&quick, &exact.rgba, size[0] as usize, size[1] as usize);
                // How far the processor's own picture is from itself with other random numbers.
                let mut other = scene.clone();
                other.seed += 1;
                let again = fractique_core::render(&other, &options, &Control::default()).expect("it renders");
                let (grain, _) = apart(&again.rgba, &exact.rgba, size[0] as usize, size[1] as usize);
                println!("| {name} | {mean:.2} | {largest:.1} | {grain:.2} | {first:.3} | {full:.3} | {slow:.2} |");
                let batches: Vec<String> = card
                    .chaos
                    .batches()
                    .iter()
                    .map(|(took, steps)| format!("{:.0}/{:.1}", took * 1000.0, *steps as f64 / 1e6))
                    .collect();
                eprintln!("batches, ms/million steps: {}", batches.join(" "));
                if let Some(folder) = &folder {
                    let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                    save(
                        &folder.join(format!("deep-{preset}-{by}.png")),
                        &both,
                        size[0],
                        size[1] * 2,
                    );
                }
            }
        }
    }

    #[test]
    fn a_picture_the_card_is_late_with_costs_that_picture_only() {
        let Some(mut card) = card() else {
            return;
        };
        let options = options(320, 200);
        let late = presets::get("solar-mandala").expect("the preset exists");
        let patient = card.chaos.longest.swap(0, Ordering::Relaxed);
        assert!(card.drawn(&late, &options).is_none());
        card.chaos.longest.store(patient, Ordering::Relaxed);
        assert!(card.chaos.works());
        assert!(card.drawn(&late, &options).is_none());
        let other = presets::get("aurora-spiral").expect("the preset exists");
        assert!(card.drawn(&other, &options).is_some());
    }

    // Draws three deep views with particles and, with as many points in the picture, with the plain chaos game,
    // which takes the card a minute, and prints how far each is from the other and from the processor's.
    #[test]
    #[ignore = "a report, not a check"]
    fn a_deep_view_beside_the_plain_chaos_game() {
        let mut card = card().expect("a graphics card with compute shaders");
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let options = options(640, 360);
        println!("| Scene | Points in view | Card to plain | Processor to plain | Card to processor |");
        println!("| --- | --- | --- | --- | --- |");
        for (preset, by) in [
            ("aurora-spiral", 14.0),
            ("aurora-spiral", 100.0),
            ("ember-vortex", 14.0),
        ] {
            let scene = magnified(preset, by);
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let (quick, _, _) = card
                .drawn(&scene, &options)
                .expect("the graphics card draws a deep view");
            let plan = |run: &Run| {
                let plan = run.layers[0].deep.as_ref()?;
                Some((plan.exposure, plan.share()))
            };
            let (exposure, share) = card.chaos.read(plan).flatten().expect("the view is deep");
            card.chaos.plain = Some(exposure);
            let all = RenderOptions {
                samples_per_pixel: (32.0 / share).min(3.0e5) as u32,
                ..options
            };
            let (plain, _, full) = card
                .drawn(&scene, &all)
                .expect("the graphics card plays the chaos game");
            card.chaos.plain = None;
            let (deep, _) = apart(&quick, &plain, 640, 360);
            let (processor, _) = apart(&exact.rgba, &plain, 640, 360);
            let (both, _) = apart(&quick, &exact.rgba, 640, 360);
            println!("| {preset} × {by} | {share:.1e} | {deep:.2} | {processor:.2} | {both:.2} |");
            eprintln!("the plain chaos game took {full:.1} s");
            if let Some(folder) = &folder {
                let three: Vec<u8> = quick.iter().chain(&plain).chain(&exact.rgba).copied().collect();
                save(&folder.join(format!("plain-{preset}-{by}.png")), &three, 640, 1080);
            }
        }
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

        for kind in ["mandelbulb", "hopalong", "buddhabrot", "gumowski_mira"] {
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

    pub(crate) fn save(path: &std::path::Path, rgba: &[u8], width: u32, height: u32) {
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
