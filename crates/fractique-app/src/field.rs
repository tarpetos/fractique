use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_wgpu::wgpu;
use fractique_core::{Camera, Fractal, RenderOptions, fractal, math, palette, scene};

use web_time::Instant;

use crate::pace;
use crate::preview::{LEVELS, guarded};
use crate::sheets::Stacks;

const HEAD: &str = include_str!("field.wgsl");
const ESCAPE: &str = include_str!("field_escape.wgsl");
const ROOT: &str = include_str!("field_root.wgsl");

// However wrong the guess of a batch's time, its samples at their dearest take no longer than this.
const DEAREST: f64 = 0.1;
// Steps a second of a card nothing is known about yet: less than any card that passed the trial does.
const UNHURRIED: f64 = 5.0e7;
// Steps of the shaders' loops one submission holds at the very most, whatever the card has shown: its samples
// times the steps a sample takes at most. It is checked before anything is sent.
const MOST_WORK: f64 = 1.0e8;
// The samples of the smallest batch: one workgroup. A sample that could take more than its share of the bound
// is not sent to the card at all.
const LEAST: u64 = 64;

fn bound(rate: &Rate) -> f64 {
    let steps = if rate.steps > 0.0 { rate.steps } else { UNHURRIED };
    (steps * DEAREST).min(MOST_WORK)
}

// How many times over the smallest batch fits in a layer's share of the bound, if every sample ran to its last step.
fn room(rate: &Rate, share: f64, cost: f64) -> f64 {
    bound(rate) * share / (LEAST as f64 * cost)
}

const BAND: u32 = 8;
// The pixels one batch looks at to count those that differ from a neighbour: nine reads of a texture each.
const MOST_COUNTED: u32 = 4_000_000;

#[derive(Default)]
struct Numbers {
    p: [[f32; 4]; 20],
    steps: u32,
    // Steps of the shader's loops a sample takes at most.
    cost: f64,
}

// A kind the graphics card draws: a function of its name in the family's shader, and what turns the kind's
// parameters, in the order of its Spec, into the numbers that function reads.
struct Kind {
    name: &'static str,
    family: &'static str,
    pack: fn(&[f64], &Camera) -> Numbers,
    // Pixels a unit of the plane, times the size of the numbers at the view's far edge: above it 32-bit floats
    // show in the picture, and the processor draws the view.
    finest: f64,
}

const KINDS: &[Kind] = &[
    Kind {
        name: "burning_ship",
        family: ESCAPE,
        pack: |v, c| escape(v, c, 0, 2.0),
        finest: 4.0e5,
    },
    Kind {
        name: "julia",
        family: ESCAPE,
        pack: julia,
        finest: 4.0e5,
    },
    Kind {
        name: "magnet",
        family: ESCAPE,
        pack: magnet,
        finest: 4.0e5,
    },
    Kind {
        name: "mandelbrot",
        family: ESCAPE,
        pack: |v, c| escape(v, c, 0, 2.0),
        finest: 4.0e5,
    },
    Kind {
        name: "multibrot",
        family: ESCAPE,
        pack: multibrot,
        finest: 4.0e5,
    },
    Kind {
        name: "phoenix",
        family: ESCAPE,
        pack: phoenix,
        finest: 4.0e5,
    },
    Kind {
        name: "tricorn",
        family: ESCAPE,
        pack: |v, c| escape(v, c, 0, 2.0),
        finest: 4.0e5,
    },
    Kind {
        name: "newton",
        family: ROOT,
        pack: newton,
        finest: 2.0e6,
    },
    Kind {
        name: "nova",
        family: ROOT,
        pack: nova,
        finest: 2.0e6,
    },
];

// fractique-core's escape/mod.rs: a deeper view needs more steps.
fn steps(iterations: f64, zoom: f64) -> u32 {
    let deeper = if zoom > 1.0 { 1.0 + 0.5 * math::log10(zoom) } else { 1.0 };
    (iterations * deeper).min(4e6) as u32
}

// The family's five parameters from values[at], and the power the far orbit is raised to at every step.
fn escape(values: &[f64], camera: &Camera, at: usize, power: f64) -> Numbers {
    let [iterations, density, shift, edge, fade] = [0, 1, 2, 3, 4].map(|index| values[at + index]);
    let limit = steps(iterations, camera.zoom);
    let mut numbers = Numbers {
        steps: limit,
        cost: f64::from(limit),
        ..Numbers::default()
    };
    let reach = if fade > 0.0 { 0.02 / (fade * fade) } else { 0.0 };
    numbers.p[0] = [density, shift, 0.012 * edge * edge, reach].map(|value| value as f32);
    numbers.p[1] = [math::ln(power) as f32, (4.0 / camera.zoom) as f32, 0.0, 0.0];
    numbers
}

fn julia(values: &[f64], camera: &Camera) -> Numbers {
    let mut numbers = escape(values, camera, 2, 2.0);
    numbers.p[2] = [values[0] as f32, values[1] as f32, 0.0, 0.0];
    numbers
}

fn multibrot(values: &[f64], camera: &Camera) -> Numbers {
    let power = values[0];
    let mut numbers = escape(values, camera, 1, power);
    let times = if power == power.round() { power - 2.0 } else { -1.0 };
    numbers.cost *= times.max(0.0) + 1.0;
    numbers.p[2] = [power as f32, times as f32, 0.0, 0.0];
    numbers
}

fn phoenix(values: &[f64], camera: &Camera) -> Numbers {
    let mut numbers = escape(values, camera, 4, 2.0);
    let form = if values[0] >= 0.5 { 1.0 } else { 0.0 };
    numbers.p[2] = [form, values[1] as f32, values[2] as f32, values[3] as f32];
    numbers
}

fn magnet(values: &[f64], camera: &Camera) -> Numbers {
    let mut numbers = escape(values, camera, 1, 2.0);
    let second = values[0] >= 1.5;
    numbers.p[2] = [
        if second { 1.0 } else { 0.0 },
        math::ln(if second { 3.0 } else { 2.0 }) as f32,
        0.0,
        0.0,
    ];
    numbers
}

type Complex = (f64, f64);

fn mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn div(a: Complex, b: Complex) -> Complex {
    let size = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / size, (a.1 * b.0 - a.0 * b.1) / size)
}

// fractique-core's root/newton.rs: its named polynomials, from the highest power down, and their roots.
const POLYNOMIALS: [&[f64]; 5] = [
    &[1.0, 0.0, 0.0, 0.0, -2.0, 0.0, 2.0, 0.0],
    &[
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 15.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -16.0, 0.0,
    ],
    &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
    &[1.0, 0.0, 0.0, 0.0, 0.0, -3.0, -5.0, -2.0, 3.0, 0.0, 1.0, 0.0],
    &[1.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, -4.0, 0.0],
];

fn polynomial(which: f64, degree: f64) -> Vec<Complex> {
    match which as usize {
        0 => {
            let mut all = vec![(0.0, 0.0); degree as usize + 1];
            (all[0], all[degree as usize]) = ((1.0, 0.0), (-1.0, 0.0));
            all
        }
        named => POLYNOMIALS[(named - 1).min(POLYNOMIALS.len() - 1)]
            .chunks_exact(2)
            .map(|term| (term[0], term[1]))
            .collect(),
    }
}

fn valued(polynomial: &[Complex], z: Complex) -> (Complex, Complex) {
    let (mut value, mut slope) = (polynomial[0], (0.0, 0.0));
    for term in &polynomial[1..] {
        let (risen, carried) = (mul(slope, z), mul(value, z));
        slope = (risen.0 + value.0, risen.1 + value.1);
        value = (carried.0 + term.0, carried.1 + term.1);
    }
    (value, slope)
}

fn roots(polynomial: &[Complex]) -> Vec<Complex> {
    let mut roots = Vec::with_capacity(polynomial.len() - 1);
    let mut start = (1.0, 0.0);
    for _ in 1..polynomial.len() {
        roots.push(start);
        start = mul(start, (0.4, 0.9));
    }
    for _ in 0..400 {
        for at in 0..roots.len() {
            let mut apart = (1.0, 0.0);
            for (other, root) in roots.iter().enumerate() {
                if other != at {
                    apart = mul(apart, (roots[at].0 - root.0, roots[at].1 - root.1));
                }
            }
            let off = div(valued(polynomial, roots[at]).0, apart);
            roots[at] = (roots[at].0 - off.0, roots[at].1 - off.1);
        }
    }
    for root in &mut roots {
        for _ in 0..8 {
            let (value, slope) = valued(polynomial, *root);
            if slope.0 * slope.0 + slope.1 * slope.1 > 0.0 {
                let off = div(value, slope);
                *root = (root.0 - off.0, root.1 - off.1);
            }
        }
    }
    let angle = |root: &Complex| {
        let angle = math::atan2(root.1, root.0);
        if angle < -1e-9 { angle + math::TAU } else { angle }
    };
    roots.sort_by(|a, b| angle(a).total_cmp(&angle(b)));
    roots
}

fn newton(values: &[f64], camera: &Camera) -> Numbers {
    let polynomial = polynomial(values[0], values[1]);
    let roots = roots(&polynomial);
    let limit = steps(values[4], camera.zoom);
    let mut numbers = Numbers {
        steps: limit,
        cost: f64::from(limit) * roots.len() as f64,
        ..Numbers::default()
    };
    let (relaxation, twist) = (values[2], values[3]);
    let left = (1.0 - relaxation) * (1.0 - relaxation) + twist * twist;
    let half = 6.0 * (-4.0 / math::ln(left.max(1e-9))).max(1.0);
    let squares = if (relaxation, twist) == (1.0, 0.0) { 1.0 } else { 0.0 };
    numbers.p[0] = [roots.len() as f32, squares, values[7] as f32, half as f32];
    numbers.p[1] = [relaxation, twist, values[5], values[6]].map(|value| value as f32);
    for (from, list) in [(2, &polynomial), (9, &roots)] {
        for (index, number) in list.iter().enumerate() {
            let row = &mut numbers.p[from + index / 2];
            (row[index % 2 * 2], row[index % 2 * 2 + 1]) = (number.0 as f32, number.1 as f32);
        }
    }
    numbers
}

fn nova(values: &[f64], camera: &Camera) -> Numbers {
    let (degree, relaxation) = (values[1], values[2]);
    let limit = steps(values[5], camera.zoom);
    let mut numbers = Numbers {
        steps: limit,
        cost: f64::from(limit) * degree,
        ..Numbers::default()
    };
    let form = if values[0] >= 0.5 { 1.0 } else { 0.0 };
    let start = math::pow(relaxation * (degree - 1.0) / (degree - relaxation), 1.0 / degree);
    numbers.p[0] = [form, degree as f32, (relaxation / degree) as f32, values[8] as f32];
    numbers.p[1] = [values[3], values[4], values[6], values[7]].map(|value| value as f32);
    numbers.p[2] = [start as f32, 0.0, 0.0, 0.0];
    numbers
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    size: [u32; 4],
    view: [f32; 4],
    step: [f32; 4],
    look: [f32; 4],
    p: [[f32; 4]; 20],
    palette: [[f32; 4]; 256],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Batch {
    region: [u32; 4],
    stride: [u32; 4],
}

// fractique-core's pixel.rs: the samples a side a pixel is drawn from where the picture is not smooth.
fn grid(options: &RenderOptions) -> u32 {
    let base = match options.samples_per_pixel {
        0..16 => 1,
        16..64 => 2,
        64..600 => 3,
        _ => 4,
    };
    (base * options.supersample).min(8)
}

// The steps of the kind's loop the samples of a pixel take at most, for a kind the shaders know.
pub(crate) fn steps_a_pixel(fractal: &Fractal, options: &RenderOptions) -> Option<f64> {
    let plan = planned(fractal, options, true)?;
    Some(plan.cost * f64::from(plan.frame.size[2] * plan.frame.size[2]))
}

struct Plan {
    kind: usize,
    frame: Box<Frame>,
    cost: f64,
}

fn planned(fractal: &Fractal, options: &RenderOptions, unbounded: bool) -> Option<Plan> {
    let index = KINDS.iter().position(|kind| kind.name == fractal.kind)?;
    let spec = fractal::spec(&fractal.kind).filter(|spec| !spec.is_density())?;
    let kind = &KINDS[index];
    let camera = &fractal.camera;
    let numbers = (kind.pack)(&spec.values(fractal), camera);
    let (wide, high) = (f64::from(options.width), f64::from(options.height));
    let per_unit = camera.zoom * wide.min(high) / 4.0;
    let far = (camera.center[0] * camera.center[0] + camera.center[1] * camera.center[1]).sqrt()
        + 0.5 * (wide * wide + high * high).sqrt() / per_unit;
    let fine = per_unit * far.max(1.0);
    if !unbounded && (fine.is_nan() || fine > kind.finest) {
        return None;
    }
    let up = if camera.flip_y { -per_unit } else { per_unit };
    let angle = camera.rotation.to_radians();
    let (cos, sin) = (math::cos(angle), math::sin(angle));
    let step = [cos / per_unit, sin / per_unit, sin / up, -cos / up];
    let mut frame = Box::new(Frame {
        size: [options.width, options.height, grid(options), numbers.steps],
        view: [
            camera.center[0] as f32,
            camera.center[1] as f32,
            (1.0 / per_unit) as f32,
            (2.5 / fractal.tone.gamma) as f32,
        ],
        step: step.map(|value| value as f32),
        look: [0.02, 0.0, 0.0, 0.0],
        p: numbers.p,
        palette: [[0.0; 4]; 256],
    });
    for (to, from) in frame.palette.iter_mut().zip(palette::lut(&fractal.palette)) {
        *to = [from[0], from[1], from[2], 255].map(|channel| f32::from(channel) / 255.0);
    }
    Some(Plan {
        kind: index,
        frame,
        cost: numbers.cost.max(1.0),
    })
}

// The layer is drawn pixel by pixel: the preview shows its levels as they are, without a flame's tone.
pub(crate) fn plain(layer: &scene::Layer) -> bool {
    layer
        .fractal()
        .and_then(|fractal| fractal::spec(&fractal.kind))
        .is_some_and(|spec| !spec.is_density())
}

// The graphics card has a shader for the layer.
pub(crate) fn drawn(layer: &scene::Layer) -> bool {
    layer
        .fractal()
        .is_some_and(|fractal| KINDS.iter().any(|kind| kind.name == fractal.kind))
}

// What the levels of such a layer depend on besides the fractal: its gamma is in them already.
pub(crate) fn key(layer: &scene::Layer) -> String {
    if plain(layer) {
        format!(" {}", layer.tone().gamma)
    } else {
        String::new()
    }
}

struct Pipes {
    first: wgpu::ComputePipeline,
    refine: wgpu::ComputePipeline,
    count: wgpu::ComputePipeline,
    gauge: wgpu::ComputePipeline,
}

struct Layouts {
    first: wgpu::BindGroupLayout,
    refine: wgpu::BindGroupLayout,
}

impl Layouts {
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
        let stored = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: LEVELS,
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let read = wgpu::BindingType::Texture {
            multisampled: false,
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let layout = |last| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fractique field"),
                entries: &[
                    entry(0, uniform),
                    entry(1, uniform),
                    entry(2, written),
                    entry(3, stored),
                    last,
                ],
            })
        };
        Self {
            first: layout(entry(4, stored)),
            refine: layout(entry(5, read)),
        }
    }
}

impl Pipes {
    fn new(device: &wgpu::Device, layouts: &Layouts, kind: &Kind) -> Self {
        let glue = format!("fn field(at: vec2<f32>) -> vec2<f32> {{ return {}(at); }}\n", kind.name);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(kind.name),
            source: wgpu::ShaderSource::Wgsl([HEAD, kind.family, &glue].concat().into()),
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, entry_point: &str| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(kind.name),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(kind.name),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            first: pipeline(&layouts.first, "first"),
            refine: pipeline(&layouts.refine, "refine"),
            count: pipeline(&layouts.refine, "count"),
            gauge: pipeline(&layouts.first, "gauge"),
        }
    }
}

// How fast the card has drawn a kind: what the next batch is sized by.
#[derive(Clone, Copy, Default)]
struct Rate {
    samples: f64,
    steps: f64,
    gauged: bool,
}

struct Sheet {
    size: [u32; 2],
    centers: wgpu::TextureView,
    levels: wgpu::TextureView,
}

// What the kinds drawn pixel by pixel share on the graphics card: their shaders, compiled when a kind is first
// asked for, how fast each has been, and the textures of the picture before this one.
pub(crate) struct Fields {
    device: wgpu::Device,
    queue: wgpu::Queue,
    failed: Arc<AtomicBool>,
    able: bool,
    layouts: Arc<Layouts>,
    pipes: Arc<Vec<OnceLock<Pipes>>>,
    asked: Mutex<Vec<bool>>,
    rates: Arc<Mutex<Vec<Rate>>>,
    spare: Arc<Mutex<Vec<Sheet>>>,
    pub(crate) stacks: Stacks,
    pub(crate) unbounded: bool,
}

pub(crate) enum Planned {
    No,
    Soon,
    Ready(Box<Layer>),
}

#[derive(Clone, Copy)]
enum Pass {
    Gauge,
    First,
    Count,
    Refine,
}

#[derive(Clone, Copy)]
enum Stage {
    Start,
    Gauge,
    First([u32; 2]),
    // The rows from this one down are counted next.
    Count(u32),
    // So many rows from this one down are drawn again in one batch: their pixels that differ were counted.
    Sure(u32, u32),
    // A band with more pixels that differ than one batch holds: in pieces, up to the row that ends it.
    Refine([u32; 2], u32),
    Done,
}

pub(crate) struct Layer {
    queue: wgpu::Queue,
    pipes: Arc<Vec<OnceLock<Pipes>>>,
    rates: Arc<Mutex<Vec<Rate>>>,
    spare: Arc<Mutex<Vec<Sheet>>>,
    kind: usize,
    size: [u32; 2],
    side: u32,
    cost: f64,
    sheet: Option<Sheet>,
    batch: wgpu::Buffer,
    counter: wgpu::Buffer,
    first: wgpu::BindGroup,
    refine: wgpu::BindGroup,
    stage: Stage,
    // The samples of the batch on its way, or the rows it counts, and its pass.
    flying: Option<(u64, Pass)>,
    drew: bool,
    // The part of the bound that is this layer's: one over the layers of the scene.
    share: f64,
    last: Option<(u64, f64)>,
    gauged: u32,
    // The rows counted at once.
    span: u32,
}

impl Fields {
    pub(crate) fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        failed: Arc<AtomicBool>,
    ) -> Self {
        let stored = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING;
        let allowed = |format| {
            adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(stored)
        };
        let able = device.limits().max_storage_textures_per_shader_stage >= 2 && allowed(LEVELS);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            stacks: Stacks::new(adapter, device, queue, failed.clone()),
            failed,
            able,
            layouts: Arc::new(Layouts::new(device)),
            pipes: Arc::new(KINDS.iter().map(|_| OnceLock::new()).collect()),
            asked: Mutex::new(vec![false; KINDS.len()]),
            rates: Arc::new(Mutex::new(vec![Rate::default(); KINDS.len()])),
            spare: Arc::default(),
            unbounded: false,
        }
    }

    // A shader is being compiled: the kind that asked for it cannot be drawn yet, and soon can.
    pub(crate) fn pending(&self) -> bool {
        let asked = self.asked.lock().unwrap_or_else(|e| e.into_inner());
        let kinds = asked.iter().zip(self.pipes.iter());
        self.stacks.pending() || kinds.into_iter().any(|(asked, pipes)| *asked && pipes.get().is_none())
    }

    fn compile(&self, kind: usize, ctx: &egui::Context) {
        let mut asked = self.asked.lock().unwrap_or_else(|e| e.into_inner());
        if std::mem::replace(&mut asked[kind], true) {
            return;
        }
        let (device, failed, ctx) = (self.device.clone(), self.failed.clone(), ctx.clone());
        let (layouts, pipes) = (self.layouts.clone(), self.pipes.clone());
        let build = move || {
            let made = guarded(&device, &failed, &ctx, || Pipes::new(&device, &layouts, &KINDS[kind]));
            if !failed.load(Ordering::Relaxed) {
                let _ = pipes[kind].set(made);
            }
            ctx.request_repaint();
        };
        // A driver takes up to seconds to compile a shader: the window does not wait for it.
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(build);
        #[cfg(target_arch = "wasm32")]
        build();
    }

    fn sheet(&self, size: [u32; 2]) -> Sheet {
        let mut spare = self.spare.lock().unwrap_or_else(|e| e.into_inner());
        spare.retain(|sheet| sheet.size == size);
        if let Some(sheet) = spare.pop() {
            return sheet;
        }
        let texture = || {
            self.device
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
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Sheet {
            size,
            centers: texture(),
            levels: texture(),
        }
    }

    fn plan(&self, fractal: &Fractal, options: &RenderOptions, layers: usize) -> Option<Plan> {
        if !self.able || options.supersample != 1 {
            return None;
        }
        let plan = planned(fractal, options, self.unbounded)?;
        // A pixel whose grid of samples could walk more steps than the smallest batch may hold is the processor's.
        let share = 1.0 / layers.max(1) as f64;
        let rate = self.rates.lock().unwrap_or_else(|e| e.into_inner())[plan.kind];
        let grid = f64::from(plan.frame.size[2] * plan.frame.size[2]);
        (room(&rate, share, plan.cost * grid) >= 1.0).then_some(plan)
    }

    pub(crate) fn draws(&self, fractal: &Fractal, options: &RenderOptions, layers: usize) -> bool {
        self.plan(fractal, options, layers).is_some()
    }

    // The layer made ready for the card, if the card draws this fractal at this size and depth.
    // `layers` is how many layers of the scene share one submission.
    pub(crate) fn layer(
        &self,
        fractal: &Fractal,
        options: &RenderOptions,
        layers: usize,
        ctx: &egui::Context,
    ) -> Planned {
        let Some(plan) = self.plan(fractal, options, layers) else {
            return Planned::No;
        };
        if self.pipes[plan.kind].get().is_none() {
            self.compile(plan.kind, ctx);
            return Planned::Soon;
        }
        let size = [options.width, options.height];
        let sheet = self.sheet(size);
        let buffer = |size: u64, usage| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractique field"),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let frame = buffer(size_of::<Frame>() as u64, wgpu::BufferUsages::UNIFORM);
        self.queue.write_buffer(&frame, 0, bytemuck::bytes_of(&*plan.frame));
        let batch = buffer(size_of::<Batch>() as u64, wgpu::BufferUsages::UNIFORM);
        let counter = buffer(16, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let group = |layout: &wgpu::BindGroupLayout, binding: u32| {
            fn whole(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
                wgpu::BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                }
            }
            fn view(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
                wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::TextureView(view),
                }
            }
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fractique field"),
                layout,
                entries: &[
                    whole(0, &frame),
                    whole(1, &batch),
                    whole(2, &counter),
                    view(3, &sheet.levels),
                    view(binding, &sheet.centers),
                ],
            })
        };
        Planned::Ready(Box::new(Layer {
            queue: self.queue.clone(),
            pipes: self.pipes.clone(),
            rates: self.rates.clone(),
            spare: self.spare.clone(),
            kind: plan.kind,
            size,
            side: plan.frame.size[2],
            cost: plan.cost,
            first: group(&self.layouts.first, 4),
            refine: group(&self.layouts.refine, 5),
            sheet: Some(sheet),
            batch,
            counter,
            stage: Stage::Start,
            flying: None,
            drew: false,
            share: 1.0 / layers.max(1) as f64,
            last: None,
            gauged: 0,
            span: u32::MAX,
        }))
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        let mut spare = self.spare.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sheet) = self.sheet.take()
            && spare.len() < 4
        {
            spare.push(sheet);
        }
    }
}

// What is told of a submission: the thread that waits for the graphics card takes it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type Waiter = std::sync::mpsc::Sender<wgpu::SubmissionIndex>;
#[cfg(target_arch = "wasm32")]
pub(crate) type Waiter = ();

#[derive(Clone, Copy, Default)]
pub(crate) struct Report {
    pub(crate) batches: u32,
    pub(crate) slowest: f32,
    pub(crate) spent: f32,
    pub(crate) first: Option<f32>,
    pub(crate) full: Option<f32>,
    pub(crate) late: bool,
}

// What a chain sends batch after batch: a layer drawn pixel by pixel, or a wallpaper.
pub(crate) trait Batched: wgpu::WasmNotSend {
    fn send(&mut self, encoder: &mut wgpu::CommandEncoder);
    fn took(&mut self, counted: u32, seconds: f32);
    fn counter(&self) -> &wgpu::Buffer;
    fn drew(&self) -> bool;
    fn done(&self) -> bool;
}

struct Going {
    layers: Vec<Box<dyn Batched>>,
    started: Instant,
    submitted: Instant,
    busy: bool,
    stopped: bool,
    fresh: bool,
    report: Report,
}

// The batches of a picture drawn pixel by pixel, one after the other: the end of one sends the next, so that
// none waits for a frame of the window.
#[derive(Clone)]
pub(crate) struct Chain {
    going: Arc<Mutex<Going>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    staging: wgpu::Buffer,
    failed: Arc<AtomicBool>,
    ctx: egui::Context,
    waiter: Waiter,
}

impl Chain {
    pub(crate) fn new(
        layers: Vec<Box<dyn Batched>>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        failed: &Arc<AtomicBool>,
        ctx: &egui::Context,
        waiter: Waiter,
        started: Instant,
    ) -> Self {
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fractique field"),
            size: 4 * layers.len() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            going: Arc::new(Mutex::new(Going {
                layers,
                started,
                submitted: started,
                busy: false,
                stopped: false,
                fresh: false,
                report: Report::default(),
            })),
            device: device.clone(),
            queue: queue.clone(),
            staging,
            failed: failed.clone(),
            ctx: ctx.clone(),
            waiter,
        }
    }

    pub(crate) fn busy(&self) -> bool {
        self.going.lock().unwrap_or_else(|e| e.into_inner()).busy
    }

    pub(crate) fn stop(&self) {
        self.going.lock().unwrap_or_else(|e| e.into_inner()).stopped = true;
    }

    // How far the picture is, and whether a batch has landed since this was asked last.
    pub(crate) fn look(&self) -> (Report, bool) {
        let mut going = self.going.lock().unwrap_or_else(|e| e.into_inner());
        (going.report, std::mem::take(&mut going.fresh))
    }

    // Sends the next batch, unless one is on its way or nothing is left to draw.
    pub(crate) fn go(&self) {
        let mut going = self.going.lock().unwrap_or_else(|e| e.into_inner());
        let over =
            going.stopped || self.failed.load(Ordering::Relaxed) || going.layers.iter().all(|layer| layer.done());
        if going.busy || over {
            return;
        }
        // wgpu refuses to configure the window's surface while another thread submits. The surface may be
        // configured by the very thread that is here, when wgpu tells it of a finished batch from within: the
        // next frame sends the batch then.
        let _gate = match eframe::egui_wgpu::SURFACE_GATE.try_read() {
            Ok(gate) => gate,
            Err(std::sync::TryLockError::Poisoned(gate)) => gate.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return,
        };
        let commands = guarded(&self.device, &self.failed, &self.ctx, || {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fractique field"),
            });
            for (index, layer) in going.layers.iter_mut().enumerate() {
                if !layer.done() {
                    layer.send(&mut encoder);
                    encoder.copy_buffer_to_buffer(layer.counter(), 0, &self.staging, 4 * index as u64, 4);
                }
            }
            encoder.finish()
        });
        // Before the submission: a graphics card that is the processor in disguise works inside it.
        going.submitted = Instant::now();
        going.busy = true;
        // A submission tells of batches that have finished: nothing of this run is held meanwhile.
        drop(going);
        let index = guarded(&self.device, &self.failed, &self.ctx, || self.queue.submit([commands]));
        let chain = self.clone();
        self.staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |mapped| chain.landed(mapped.is_ok()));
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.waiter.send(index);
        #[cfg(target_arch = "wasm32")]
        let _ = (index, self.waiter);
    }

    fn landed(&self, mapped: bool) {
        let counted: Option<Vec<u32>> = mapped
            .then(|| self.staging.slice(..).get_mapped_range().ok())
            .flatten()
            .map(|view| bytemuck::pod_collect_to_vec(&view));
        if mapped {
            self.staging.unmap();
        }
        {
            let mut going = self.going.lock().unwrap_or_else(|e| e.into_inner());
            going.busy = false;
            let Some(counted) = counted else {
                self.failed.store(true, Ordering::Relaxed);
                going.stopped = true;
                drop(going);
                self.ctx.request_repaint();
                return;
            };
            let took = going.submitted.elapsed().as_secs_f32().max(1e-4);
            for (layer, counted) in going.layers.iter_mut().zip(&counted) {
                layer.took(*counted, took);
            }
            let since = going.started.elapsed().as_secs_f32();
            going.report.batches += 1;
            going.report.slowest = going.report.slowest.max(took);
            going.report.spent += took;
            // The shaders are bounded in steps, not in time: a card this slow at this picture is not given
            // another batch of it.
            if took * 1000.0 > crate::chaos::LONGEST as f32 {
                log::warn!("the graphics card took {took:.2} s for one batch of a fractal: the picture is given up");
                going.report.late = true;
                going.stopped = true;
            }
            if going.layers.iter().all(|layer| layer.drew()) {
                going.report.first.get_or_insert(since);
            }
            if going.layers.iter().all(|layer| layer.done()) {
                going.report.full.get_or_insert(since);
            }
            going.fresh = true;
        }
        self.ctx.request_repaint();
        self.go();
    }
}

// The part of the picture a batch of so many pixels draws next, in bands across it; a batch smaller than a band
// takes a piece of one. The place moves past it.
pub(crate) fn strip(at: &mut [u32; 2], pixels: u64, [width, height]: [u32; 2]) -> [u32; 4] {
    let band = u64::from(width) * u64::from(BAND);
    if at[0] == 0 && pixels >= band {
        let rows = ((pixels / band) as u32).saturating_mul(BAND).min(height - at[1]);
        let region = [0, at[1], width, rows];
        at[1] += rows;
        return region;
    }
    let across = ((pixels / u64::from(BAND * BAND)) as u32)
        .saturating_mul(BAND)
        .max(BAND)
        .min(width - at[0]);
    let region = [at[0], at[1], across, BAND.min(height - at[1])];
    at[0] += across;
    if at[0] >= width {
        *at = [0, at[1] + BAND];
    }
    region
}

impl Layer {
    pub(crate) fn levels(&self) -> &wgpu::TextureView {
        &self
            .sheet
            .as_ref()
            .expect("a layer keeps its textures while it lives")
            .levels
    }

    pub(crate) fn counter(&self) -> &wgpu::Buffer {
        &self.counter
    }

    pub(crate) fn done(&self) -> bool {
        matches!(self.stage, Stage::Done)
    }

    // The samples of the next batch: as many as would have taken the pace in the batch before, between a quarter
    // of it and four times it, and never more than the bound allows if every one of them ran to its last step.
    // The first batch of a pass goes by the speed the card has shown at the kind.
    fn budget(&self, known: f64, each: f64) -> u64 {
        let rate = self.rates.lock().unwrap_or_else(|e| e.into_inner())[self.kind];
        let most = bound(&rate) * self.share / (self.cost * each);
        let samples = match self.last {
            Some((samples, seconds)) => {
                let before = samples as f64;
                pace::next(before, seconds as f32, pace::slice())
            }
            None if known > 0.0 => known * f64::from(pace::slice()),
            None => most,
        };
        (samples.min(most) as u64).max(LEAST)
    }

    // Sends the next batch: one sample a block of pixels first, when one a pixel would take too long; samples
    // that walk every step, to learn the card's speed at the kind; one sample a pixel; then the pixels that differ
    // from a neighbour are counted, and drawn again from the grid of samples.
    pub(crate) fn send(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(pipes) = self.pipes[self.kind].get() else {
            return;
        };
        let [width, height] = self.size;
        let rate = self.rates.lock().unwrap_or_else(|e| e.into_inner())[self.kind];
        if matches!(self.stage, Stage::Start) {
            self.stage = if rate.gauged {
                Stage::First([0, 0])
            } else {
                Stage::Gauge
            };
            let budget = self.budget(rate.samples, 1.0);
            let blocks = |stride: u32| u64::from(width.div_ceil(stride)) * u64::from(height.div_ceil(stride));
            if budget < blocks(1)
                && let Some(stride) = [2, 4, 8, 16, 32, 64]
                    .into_iter()
                    .find(|stride| blocks(*stride) <= budget)
            {
                self.dispatch(encoder, &pipes.first, &self.first, [0, 0, width, height], stride);
                self.flying = Some((blocks(stride), Pass::First));
                return;
            }
        }
        match self.stage {
            Stage::Start | Stage::Done => {}
            Stage::Gauge => {
                let groups = ((bound(&rate) * self.share / self.cost) as u64 / LEAST).max(1);
                let across = groups.min(1024);
                let down = (groups / across).min(1024);
                self.dispatch(
                    encoder,
                    &pipes.gauge,
                    &self.first,
                    [0, 0, across as u32 * 8, down as u32 * 8],
                    1,
                );
                self.flying = Some((across * down * LEAST, Pass::Gauge));
            }
            Stage::First(mut at) => {
                let region = strip(&mut at, self.budget(rate.samples, 1.0), self.size);
                self.stage = if at[1] < height {
                    Stage::First(at)
                } else if self.side > 1 {
                    Stage::Count(0)
                } else {
                    Stage::Done
                };
                self.dispatch(encoder, &pipes.first, &self.first, region, 1);
                self.flying = Some((u64::from(region[2]) * u64::from(region[3]), Pass::First));
            }
            Stage::Count(top) => {
                let most = (MOST_COUNTED / width / BAND * BAND).max(BAND);
                let rows = self.span.min(most).min(height - top);
                self.dispatch(encoder, &pipes.count, &self.refine, [0, top, width, rows], 1);
                self.flying = Some((u64::from(rows), Pass::Count));
            }
            Stage::Sure(top, rows) => {
                self.stage = self.below(top + rows);
                self.dispatch(encoder, &pipes.refine, &self.refine, [0, top, width, rows], 1);
                self.flying = Some((u64::from(width) * u64::from(rows), Pass::Refine));
            }
            Stage::Refine(mut at, end) => {
                let each = f64::from(self.side * self.side);
                let region = strip(&mut at, self.budget(0.0, each), [width, end]);
                self.stage = if at[1] < end {
                    Stage::Refine(at, end)
                } else {
                    self.below(end)
                };
                self.dispatch(encoder, &pipes.refine, &self.refine, region, 1);
                self.flying = Some((u64::from(region[2]) * u64::from(region[3]), Pass::Refine));
            }
        }
    }

    fn below(&self, top: u32) -> Stage {
        if top < self.size[1] {
            Stage::Count(top)
        } else {
            Stage::Done
        }
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        group: &wgpu::BindGroup,
        region: [u32; 4],
        stride: u32,
    ) {
        let batch = Batch {
            region,
            stride: [stride, 0, 0, 0],
        };
        self.queue.write_buffer(&self.batch, 0, bytemuck::bytes_of(&batch));
        self.queue.write_buffer(&self.counter, 0, &[0; 16]);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fractique field"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        let block = 8 * stride;
        pass.dispatch_workgroups(region[2].div_ceil(block), region[3].div_ceil(block), 1);
    }

    // Takes in how long the batch took and what it counted: a sixteenth of the steps its samples walked, or the
    // pixels that differ from a neighbour.
    pub(crate) fn took(&mut self, counted: u32, seconds: f32) {
        let Some((samples, pass)) = self.flying.take() else {
            return;
        };
        let seconds = f64::from(seconds).max(1e-4);
        let mut rates = self.rates.lock().unwrap_or_else(|e| e.into_inner());
        let rate = &mut rates[self.kind];
        self.last = None;
        match pass {
            Pass::Gauge => {
                rate.steps = rate.steps.max(samples as f64 * self.cost / seconds);
                self.gauged += 1;
                // A batch this long is the card's work more than the wait for it.
                if seconds >= 0.5 * f64::from(pace::slice()) || rate.steps * DEAREST >= MOST_WORK || self.gauged >= 4 {
                    rate.gauged = true;
                    self.stage = Stage::First([0, 0]);
                }
            }
            Pass::First => {
                rate.samples = if rate.samples > 0.0 {
                    0.5 * (rate.samples + samples as f64 / seconds)
                } else {
                    samples as f64 / seconds
                };
                self.drew = true;
                rate.steps = rate.steps.max(f64::from(counted) * 16.0 / seconds);
                self.last = Some((samples, seconds));
            }
            Pass::Count => {
                let Stage::Count(top) = self.stage else {
                    return;
                };
                let rows = samples as u32;
                let worst = f64::from(counted) * f64::from(self.side * self.side) * self.cost;
                let allowed = bound(rate) * self.share;
                self.stage = if counted == 0 {
                    self.below(top + rows)
                } else if worst <= allowed {
                    if 2.0 * worst <= allowed {
                        self.span = self.span.saturating_mul(2);
                    }
                    Stage::Sure(top, rows)
                } else if rows > BAND {
                    self.span = ((f64::from(rows) * allowed / worst) as u32 / BAND * BAND).max(BAND);
                    Stage::Count(top)
                } else {
                    Stage::Refine([0, top], top + rows)
                };
            }
            Pass::Refine => {
                rate.steps = rate.steps.max(f64::from(counted) * 16.0 / seconds);
                if matches!(self.stage, Stage::Refine(..)) {
                    self.last = Some((samples, seconds));
                }
            }
        }
    }

    // A picture of the layer is in its levels.
    pub(crate) fn drew(&self) -> bool {
        self.drew
    }
}

impl Batched for Layer {
    fn send(&mut self, encoder: &mut wgpu::CommandEncoder) {
        Layer::send(self, encoder);
    }

    fn took(&mut self, counted: u32, seconds: f32) {
        Layer::took(self, counted, seconds);
    }

    fn counter(&self) -> &wgpu::Buffer {
        Layer::counter(self)
    }

    fn drew(&self) -> bool {
        Layer::drew(self)
    }

    fn done(&self) -> bool {
        Layer::done(self)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fractique_core::{Control, Scene};
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

    fn size() -> [u32; 2] {
        let asked: Vec<u32> = std::env::var("FRACTIQUE_COMPARE_SIZE")
            .unwrap_or_else(|_| "960x540".to_owned())
            .split('x')
            .filter_map(|side| side.parse().ok())
            .collect();
        [asked[0], asked[1]]
    }

    #[test]
    fn every_pixelwise_kind_of_the_core_has_a_shader() {
        for spec in fractal::all().filter(|spec| !spec.is_density()) {
            let family = spec.family.name();
            let ported = ["escape-time", "root-finding"].contains(&family);
            assert_eq!(KINDS.iter().any(|kind| kind.name == spec.name), ported, "{}", spec.name);
        }
        for kind in KINDS {
            let source = [HEAD, kind.family].concat();
            assert!(source.contains(&format!("fn {}(", kind.name)), "{}", kind.name);
        }
    }

    #[test]
    fn a_scene_that_could_take_more_than_the_bound_is_not_for_the_card() {
        let quick = Rate {
            steps: 1.0e12,
            ..Rate::default()
        };
        let slow = Rate {
            steps: 1.0e6,
            ..Rate::default()
        };
        for (rate, most) in [
            (Rate::default(), UNHURRIED * DEAREST),
            (quick, MOST_WORK),
            (slow, 1.0e6 * DEAREST),
        ] {
            let cost = most / LEAST as f64;
            assert!(room(&rate, 1.0, cost) >= 1.0);
            assert!(room(&rate, 1.0, cost * 1.01) < 1.0);
            assert!(room(&rate, 0.5, cost) < 1.0);
            assert!(LEAST as f64 * cost <= MOST_WORK);
        }
        let mut scene = fractal::preset("mandelbrot-whole").expect("a preset");
        let fractal = scene.layers[0].fractal_mut().expect("a fractal");
        fractal.params.insert("iterations".to_owned(), 100_000.0);
        let plan = planned(fractal, &options(320, 200), false).expect("the kind has a shader");
        assert!(room(&Rate::default(), 1.0, plan.cost) < 1.0);
    }

    #[test]
    fn a_batch_walks_the_picture_in_bands_and_pieces() {
        let size = [100, 30];
        let mut at = [0, 0];
        let mut covered = 0;
        for pixels in [64, 64, 100_000, 900, 1700, 1, 1_000_000] {
            let [left, top, across, down] = strip(&mut at, pixels, size);
            assert!(
                left + across <= 100 && across > 0 && down > 0,
                "{left} {top} {across} {down}"
            );
            covered += across * down.min(30 - top);
            if at[1] >= 30 {
                break;
            }
        }
        assert_eq!(covered, 100 * 30);
        assert!(at[1] >= 30);
    }

    #[test]
    fn the_graphics_card_draws_the_fractals_the_processor_renders() {
        let Some(mut card) = card() else {
            eprintln!("no graphics card with compute shaders: not compared");
            return;
        };
        let options = options(480, 270);
        for name in ["mandelbrot-whole", "julia-dragon", "newton-cubic", "nova-whole"] {
            let Some(scene) = fractal::preset(name) else {
                panic!("{name} is a preset");
            };
            let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
            let (quick, _, _) = card.drawn(&scene, &options).expect("the card draws it");
            let (mean, largest) = apart(&quick, &exact.rgba, 480, 270);
            assert!(mean < 1.0 && largest < 40.0, "{name}: {mean} {largest}");
        }
    }

    #[test]
    fn layers_of_fractals_are_blended_as_the_processor_blends_them() {
        let Some(mut card) = card() else {
            eprintln!("no graphics card with compute shaders: not compared");
            return;
        };
        let options = options(320, 180);
        let mut scene = fractal::preset("julia-dragon").expect("a preset");
        let mut over = fractal::preset("newton-cubic").expect("a preset").layers.remove(0);
        over.blend = fractique_core::Blend::Screen;
        over.opacity = 0.6;
        scene.layers.push(over);
        let exact = fractique_core::render(&scene, &options, &Control::default()).expect("it renders");
        let (quick, _, _) = card.drawn(&scene, &options).expect("the card draws it");
        let (mean, largest) = apart(&quick, &exact.rgba, 320, 180);
        assert!(mean < 1.0 && largest < 40.0, "{mean} {largest}");
        scene.layers.push(
            fractique_core::presets::get("aurora-spiral")
                .expect("a preset")
                .layers
                .remove(0),
        );
        assert!(card.drawn(&scene, &options).is_none());
    }

    fn compared(card: &mut crate::chaos::tests::Card, name: &str, scene: &Scene, folder: Option<&PathBuf>) {
        let [width, height] = size();
        let options = options(width, height);
        let started = Instant::now();
        let exact = fractique_core::render(scene, &options, &Control::default()).expect("it renders");
        let slow = started.elapsed().as_secs_f32();
        let Some((quick, first, full)) = card.drawn(scene, &options) else {
            println!("| {name} | not drawn | | | | {slow:.2} |");
            return;
        };
        let (mean, largest) = apart(&quick, &exact.rgba, width as usize, height as usize);
        println!("| {name} | {mean:.2} | {largest:.1} | {first:.3} | {full:.3} | {slow:.2} |");
        if let Some(folder) = folder {
            let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
            save(&folder.join(format!("{name}.png")), &both, width, height * 2);
        }
    }

    // FRACTIQUE_COMPARE=folder cargo test --release -p fractique-app --lib every_fractal -- --ignored --nocapture
    // writes the table of differences, and for every scene the picture of the graphics card above the processor's.
    #[test]
    #[ignore = "a report, not a check"]
    fn every_fractal_on_the_graphics_card_beside_the_processor() {
        let mut card = card().expect("a graphics card with compute shaders");
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        let seeds: u64 = std::env::var("FRACTIQUE_COMPARE_SEEDS")
            .ok()
            .and_then(|seeds| seeds.parse().ok())
            .unwrap_or(10);
        println!("| Scene | Mean | Largest | First picture, s | Whole picture, s | Processor, s |");
        println!("| --- | --- | --- | --- | --- | --- |");
        let mut scenes: Vec<(String, Scene)> = fractal::preset_names()
            .into_iter()
            .filter_map(|name| Some((name.clone(), fractal::preset(&name)?)))
            .collect();
        for kind in KINDS {
            for seed in 1..=seeds {
                let scene = fractal::random_scene(kind.name, seed).expect("the kind is listed");
                scenes.push((format!("{}-random-{seed}", kind.name), scene));
            }
        }
        for (name, scene) in scenes {
            let ported = scene.layers.iter().all(drawn);
            if ported && only.as_ref().is_none_or(|only| name.contains(only.as_str())) {
                compared(&mut card, &name, &scene, folder.as_ref());
            }
        }
    }

    // How far 32-bit floats reach: a view of every kind magnified step by step, with the limit taken away.
    #[test]
    #[ignore = "a report, not a check"]
    fn how_deep_single_precision_reaches() {
        let mut card = card().expect("a graphics card with compute shaders");
        card.chaos.unbound(true);
        let folder = std::env::var_os("FRACTIQUE_COMPARE").map(PathBuf::from);
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        let [width, height] = size();
        println!("| Scene | Magnified | Pixels a unit times the far edge | Mean | Largest |");
        println!("| --- | --- | --- | --- | --- |");
        for kind in KINDS {
            if only.as_ref().is_some_and(|only| !kind.name.contains(only.as_str())) {
                continue;
            }
            for seed in 1..=3 {
                let scene = fractal::random_scene(kind.name, seed).expect("the kind is listed");
                let mut by = 1.0;
                while by < 3.0e6 {
                    let mut deeper = scene.clone();
                    let camera = deeper.layers[0].camera_mut();
                    *camera = camera.looking_at(0.0, 0.0, by);
                    let (center, zoom) = (camera.center, camera.zoom);
                    let per_unit = zoom * f64::from(width.min(height)) / 4.0;
                    let far = (center[0] * center[0] + center[1] * center[1]).sqrt()
                        + 0.5 * f64::from(width * width + height * height).sqrt() / per_unit;
                    let options = options(width, height);
                    let exact = fractique_core::render(&deeper, &options, &Control::default()).expect("it renders");
                    let name = format!("{}-{seed}-x{by}", kind.name);
                    match card.drawn(&deeper, &options) {
                        Some((quick, _, _)) => {
                            let (mean, largest) = apart(&quick, &exact.rgba, width as usize, height as usize);
                            println!(
                                "| {}-{seed} | {by} | {:.3e} | {mean:.2} | {largest:.1} |",
                                kind.name,
                                per_unit * far.max(1.0)
                            );
                            if let Some(folder) = &folder {
                                let both: Vec<u8> = quick.iter().chain(&exact.rgba).copied().collect();
                                save(&folder.join(format!("{name}.png")), &both, width, height * 2);
                            }
                        }
                        None => println!("| {}-{seed} | {by} | | not drawn | |", kind.name),
                    }
                    by *= 4.0;
                }
            }
        }
    }

    // The seconds to the first and to the whole picture of the first preset of every kind, at the sizes given
    // in FRACTIQUE_SPEED_SIZES (1920x1080,3840x2160), beside the processor's draft and full picture.
    #[test]
    #[ignore = "a report, not a check"]
    fn how_fast_the_graphics_card_draws_fractals() {
        let mut card = card().expect("a graphics card with compute shaders");
        let sizes = std::env::var("FRACTIQUE_SPEED_SIZES").unwrap_or_else(|_| "1920x1080,3840x2160".to_owned());
        let only = std::env::var("FRACTIQUE_COMPARE_ONLY").ok();
        println!("| Scene | Size | Card: first, s | Card: whole, s | Processor: draft, s | Processor: whole, s |");
        println!("| --- | --- | --- | --- | --- | --- |");
        for kind in KINDS {
            if only.as_ref().is_some_and(|only| !kind.name.contains(only.as_str())) {
                continue;
            }
            let spec = fractal::spec(kind.name).expect("the kind is listed");
            let name = format!("{}-{}", kind.name, spec.presets[0].name);
            let scene = fractal::preset(&name).expect("the preset is listed");
            for size in sizes.split(',') {
                let sides: Vec<u32> = size.split('x').filter_map(|side| side.parse().ok()).collect();
                let options = options(sides[0], sides[1]);
                let shrink = f64::from(sides[0].max(sides[1])) / 480.0;
                let draft = RenderOptions {
                    width: (f64::from(sides[0]) / shrink) as u32,
                    height: (f64::from(sides[1]) / shrink) as u32,
                    samples_per_pixel: 12,
                    supersample: 1,
                };
                let timed = |options: &RenderOptions| {
                    let started = Instant::now();
                    fractique_core::render(&scene, options, &Control::default()).expect("it renders");
                    started.elapsed().as_secs_f32()
                };
                let (rough, exact) = (timed(&draft), timed(&options));
                // The second run is the one that counts: the first compiles the shader and learns the card's pace.
                let _ = card.drawn(&scene, &options);
                match card.drawn(&scene, &options) {
                    Some((_, first, full)) => {
                        println!("| {name} | {size} | {first:.3} | {full:.3} | {rough:.3} | {exact:.3} |");
                    }
                    None => println!("| {name} | {size} | not drawn | | {rough:.3} | {exact:.3} |"),
                }
            }
        }
    }
}
