use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_wgpu::wgpu;
use fractique_core::{Blend, Density, Scene};

pub(crate) const LEVELS: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
// Not an sRGB format, like the picture the processor renders: the panes blend in gamma space.
const PICTURE: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
// A blur reaches three sigmas; above this sigma it is run on a smaller picture instead.
const WIDEST_SIGMA: f32 = 4.0;

pub struct Source {
    pub density: Density,
    waiting: Mutex<Option<Vec<u16>>>,
    levels: OnceLock<wgpu::TextureView>,
}

fn half(value: f32) -> u16 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 112;
    let mantissa = bits & 0x7f_ffff;
    if exponent >= 31 {
        return 0x7bff;
    }
    if exponent <= 0 {
        let shift = (14 - exponent) as u32;
        return if shift > 24 {
            0
        } else {
            ((mantissa | 0x80_0000) >> shift) as u16
        };
    }
    let rounded = ((exponent as u32) << 10 | mantissa >> 13) + (mantissa >> 12 & 1);
    rounded.min(0x7bff) as u16
}

impl Source {
    pub fn new(density: Density) -> Self {
        let halves = density.levels().into_iter().map(half).collect();
        Self {
            density,
            waiting: Mutex::new(Some(halves)),
            levels: OnceLock::new(),
        }
    }

    pub fn bytes(&self) -> usize {
        let waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        self.density.bytes() + waiting.as_ref().map_or(0, |halves| halves.len() * 2)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    tone: [f32; 4],
    blend: [f32; 4],
    reach: [f32; 4],
}

struct Target {
    size: [u32; 2],
    view: wgpu::TextureView,
}

pub struct Preview {
    device: wgpu::Device,
    queue: wgpu::Queue,
    largest: u32,
    failed: Arc<AtomicBool>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    layer: [wgpu::RenderPipeline; 3],
    toned: wgpu::RenderPipeline,
    shrink: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    buffers: Vec<wgpu::Buffer>,
    nothing: wgpu::TextureView,
    picture: Option<Target>,
    soft: Option<[Target; 2]>,
    glows: HashMap<[u32; 2], [Target; 2]>,
}

// An error of the graphics card inside `run` sets `failed` instead of ending the app.
pub(crate) fn guarded<T>(
    device: &wgpu::Device,
    failed: &Arc<AtomicBool>,
    ctx: &egui::Context,
    run: impl FnOnce() -> T,
) -> T {
    let scopes =
        [wgpu::ErrorFilter::OutOfMemory, wgpu::ErrorFilter::Validation].map(|filter| device.push_error_scope(filter));
    let out = run();
    for scope in scopes.into_iter().rev() {
        let mut error = Box::pin(scope.pop());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match error.as_mut().poll(&mut context) {
            std::task::Poll::Ready(None) => {}
            std::task::Poll::Ready(Some(error)) => {
                log::warn!("the graphics card refused its work: {error}");
                failed.store(true, Ordering::Relaxed);
            }
            std::task::Poll::Pending => {
                #[cfg(target_arch = "wasm32")]
                {
                    let (failed, ctx) = (failed.clone(), ctx.clone());
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(error) = error.await {
                            log::warn!("the graphics card refused its work: {error}");
                            failed.store(true, Ordering::Relaxed);
                            ctx.request_repaint();
                        }
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = ctx;
            }
        }
    }
    out
}

fn target(device: &wgpu::Device, size: [u32; 2]) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fractique preview"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: PICTURE,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    Target {
        size,
        view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
    }
}

struct Pass<'a> {
    pipeline: &'a wgpu::RenderPipeline,
    target: &'a wgpu::TextureView,
    clear: Option<[f64; 3]>,
    levels: &'a wgpu::TextureView,
    image: &'a wgpu::TextureView,
    uniforms: Uniforms,
}

impl Preview {
    pub fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ctx: &egui::Context,
    ) -> Option<Self> {
        let needed = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let drawn = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let allowed = |format| adapter.get_texture_format_features(format).allowed_usages;
        if !allowed(LEVELS).contains(needed) || !allowed(PICTURE).contains(drawn) {
            return None;
        }
        let failed = Arc::new(AtomicBool::new(false));
        let preview = guarded(device, &failed, ctx, || Self::build(device, queue, failed.clone()));
        (!failed.load(Ordering::Relaxed)).then_some(preview)
    }

    fn build(device: &wgpu::Device, queue: &wgpu::Queue, failed: Arc<AtomicBool>) -> Self {
        let texture = |binding, filterable| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fractique preview"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Read texel by texel: phones and WebGL2 do not all filter float textures.
                texture(1, false),
                texture(2, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fractique preview"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fractique preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let pipeline = |fragment: &str, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fragment),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    entry_point: Some("vs_main"),
                    module: &module,
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fragment),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: PICTURE,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let blended = |src_factor, dst_factor| {
            let component = wgpu::BlendComponent {
                src_factor,
                dst_factor,
                operation: wgpu::BlendOperation::Add,
            };
            Some(wgpu::BlendState {
                color: component,
                alpha: component,
            })
        };
        let (one, less_source, less_canvas) = (
            wgpu::BlendFactor::One,
            wgpu::BlendFactor::OneMinusSrcAlpha,
            wgpu::BlendFactor::OneMinusDst,
        );
        let nothing = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fractique preview"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PICTURE,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &nothing,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        Self {
            device: device.clone(),
            queue: queue.clone(),
            largest: device.limits().max_texture_dimension_2d,
            failed,
            layer: [
                pipeline("fs_layer", blended(one, one)),
                pipeline("fs_layer", blended(less_canvas, one)),
                pipeline("fs_layer", blended(one, less_source)),
            ],
            toned: pipeline("fs_toned", None),
            shrink: pipeline("fs_shrink", None),
            blur: pipeline("fs_blur", None),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("fractique preview"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            layout,
            buffers: Vec::new(),
            nothing: nothing.create_view(&wgpu::TextureViewDescriptor::default()),
            picture: None,
            soft: None,
            glows: HashMap::new(),
        }
    }

    pub fn works(&self) -> bool {
        !self.failed.load(Ordering::Relaxed)
    }

    // A copy of the picture and of its blurred one as they are now, for the screen to fade from while the next
    // picture is drawn into the same textures.
    pub fn snapshot(&self) -> Option<(wgpu::TextureView, wgpu::TextureView)> {
        let (picture, soft) = (self.picture.as_ref()?, &self.soft.as_ref()?[0]);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut copy = |from: &Target| {
            let size = wgpu::Extent3d {
                width: from.size[0],
                height: from.size[1],
                depth_or_array_layers: 1,
            };
            let kept = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fractique picture before"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PICTURE,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            encoder.copy_texture_to_texture(from.view.texture().as_image_copy(), kept.as_image_copy(), size);
            kept.create_view(&wgpu::TextureViewDescriptor::default())
        };
        let kept = (copy(picture), copy(soft));
        self.queue.submit([encoder.finish()]);
        Some(kept)
    }

    pub fn views(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView)> {
        Some((&self.picture.as_ref()?.view, &self.soft.as_ref()?[0].view))
    }

    // Copies the picture drawn last, or the blurred one under the panes, back from the graphics card. `then` gets
    // its bytes on the thread that hears of the submission returned here: the caller has one listen for it.
    pub fn read(
        &self,
        soft: bool,
        ctx: &egui::Context,
        then: impl FnOnce(Option<(Vec<u8>, [u32; 2])>) + Send + 'static,
    ) -> Option<wgpu::SubmissionIndex> {
        let target = if soft {
            &self.soft.as_ref()?[0]
        } else {
            self.picture.as_ref()?
        };
        let size = target.size;
        let stride = (size[0] * 4).div_ceil(256) * 256;
        let index = guarded(&self.device, &self.failed, ctx, || {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractique picture"),
                size: u64::from(stride) * u64::from(size[1]),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("fractique picture"),
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: target.view.texture(),
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
            let index = self.queue.submit([encoder.finish()]);
            let mapped = buffer.clone();
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
                let pixels = result
                    .ok()
                    .and_then(|()| mapped.slice(..).get_mapped_range().ok())
                    .map(|rows| {
                        rows.chunks(stride as usize)
                            .flat_map(|row| row[..size[0] as usize * 4].iter().copied())
                            .collect::<Vec<u8>>()
                    });
                mapped.unmap();
                then(pixels.map(|pixels| (pixels, size)));
            });
            index
        });
        self.works().then_some(index)
    }

    // The layers' levels as textures: false when one of them cannot be on this graphics card.
    pub fn take(&self, sources: &[Arc<Source>], ctx: &egui::Context) -> bool {
        let taken = guarded(&self.device, &self.failed, ctx, || {
            sources.iter().all(|source| self.upload(source))
        });
        taken && self.works()
    }

    fn upload(&self, source: &Source) -> bool {
        if source.levels.get().is_some() {
            return true;
        }
        let [width, height] = source.density.size();
        let halves = source.waiting.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(halves) = halves else { return false };
        if width.max(height) > self.largest || halves.len() != (width * height * 4) as usize {
            return false;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fractique levels"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: LEVELS,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&halves),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let _ = source
            .levels
            .set(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        true
    }

    // Draws the scene's layers with their tone, opacity and blend as they are now. The layers' places and the
    // smoothing stay those the sources were rendered with.
    pub fn draw(&mut self, scene: &Scene, sources: &[Arc<Source>], ctx: &egui::Context) -> bool {
        let Some(size) = sources.first().map(|source| source.density.size()) else {
            return false;
        };
        let levels: Option<Vec<&wgpu::TextureView>> = sources
            .iter()
            .map(|source| source.levels.get().filter(|_| source.density.size() == size))
            .collect();
        levels.is_some_and(|levels| self.draw_levels(scene, size, &levels, ctx))
    }

    // The same from levels that are on the graphics card already, one texture of this size a layer.
    pub fn draw_levels(
        &mut self,
        scene: &Scene,
        size: [u32; 2],
        levels: &[&wgpu::TextureView],
        ctx: &egui::Context,
    ) -> bool {
        let (device, failed) = (self.device.clone(), self.failed.clone());
        let drawn = guarded(&device, &failed, ctx, || self.passes(scene, size, levels).is_some());
        drawn && self.works()
    }

    fn passes(&mut self, scene: &Scene, size: [u32; 2], sources: &[&wgpu::TextureView]) -> Option<()> {
        let wallpaper = crate::sheets::drawn(scene);
        if sources.len() != scene.layers.len() + usize::from(wallpaper) {
            return None;
        }
        if self.picture.as_ref().is_none_or(|picture| picture.size != size) {
            self.picture = Some(target(&self.device, size));
            self.soft = None;
            self.glows.clear();
        }
        let shorter = size[0].min(size[1]) as f32;

        // The blurred picture under the panes, as `blur::soften` makes it: a quarter of the size, or less.
        let quarter = (shorter * 0.03 / 4.0).max(0.5);
        let fewer = (quarter / (WIDEST_SIGMA * 0.5)).floor().max(1.0);
        let small = size.map(|side| ((side as f32 / (4.0 * fewer)) as u32).max(1));
        if self.soft.is_none() {
            self.soft = Some([target(&self.device, small), target(&self.device, small)]);
        }

        let glowing: Vec<Option<(u32, f32)>> = scene
            .layers
            .iter()
            .map(|layer| {
                let tone = layer.tone();
                let sigma = (tone.glow_radius * f64::from(size[0].min(size[1]))) as f32;
                (tone.glow_strength > 0.0 && sigma >= 0.5).then(|| {
                    let shrink = (sigma / (WIDEST_SIGMA * 0.5)).floor().max(1.0);
                    (shrink as u32, sigma / shrink)
                })
            })
            .collect();
        for (shrink, _) in glowing.iter().flatten() {
            let small = size.map(|side| side.div_ceil(*shrink));
            if !self.glows.contains_key(&small) {
                let pair = [target(&self.device, small), target(&self.device, small)];
                self.glows.insert(small, pair);
            }
        }

        let picture = self.picture.as_ref()?;
        let soft = self.soft.as_ref()?;
        let mut passes = Vec::new();
        if wallpaper {
            passes.push(Pass {
                pipeline: &self.layer[2],
                target: &picture.view,
                clear: Some(scene.background),
                levels: sources[0],
                image: &self.nothing,
                uniforms: Uniforms {
                    tone: [1.0, 1.0, 0.0, 0.0],
                    blend: [1.0, 0.0, 1.0, 1.0],
                    reach: [1.0, 1.0, 0.0, 1.0],
                },
            });
        }
        for (index, ((layer, levels), glow)) in scene.layers.iter().zip(sources).zip(&glowing).enumerate() {
            let tone = layer.tone();
            let levels = *levels;
            let plain = if crate::field::plain(layer) { 1.0 } else { 0.0 };
            let mut uniforms = Uniforms {
                tone: [
                    tone.brightness as f32,
                    (1.0 / tone.gamma) as f32,
                    tone.vibrancy as f32,
                    tone.highlight as f32,
                ],
                blend: [layer.opacity as f32, 0.0, 1.0, 0.0],
                reach: [1.0, 1.0, 0.0, plain],
            };
            let mut image = &self.nothing;
            if let Some((shrink, sigma)) = *glow {
                let [first, second] = self.glows.get(&size.map(|side| side.div_ceil(shrink)))?;
                let across = shrink as f32;
                passes.push(Pass {
                    pipeline: &self.toned,
                    target: &first.view,
                    clear: Some([0.0; 3]),
                    levels,
                    image: &self.nothing,
                    uniforms: Uniforms {
                        blend: [1.0, 0.0, across.min(4.0), 0.0],
                        reach: [across, across, 0.0, plain],
                        ..uniforms
                    },
                });
                for (from, to, direction) in [(first, second, [1.0, 0.0]), (second, first, [0.0, 1.0])] {
                    passes.push(Pass {
                        pipeline: &self.blur,
                        target: &to.view,
                        clear: Some([0.0; 3]),
                        levels: &self.nothing,
                        image: &from.view,
                        uniforms: Uniforms {
                            reach: [direction[0], direction[1], sigma, 0.0],
                            ..uniforms
                        },
                    });
                }
                uniforms.blend[1] = tone.glow_strength as f32;
                uniforms.reach = [across, across, 0.0, plain];
                image = &first.view;
            }
            let blend = match layer.blend {
                Blend::Add => 0,
                Blend::Screen => 1,
                Blend::Over => 2,
            };
            passes.push(Pass {
                pipeline: &self.layer[blend],
                target: &picture.view,
                clear: (index == 0).then_some(scene.background),
                levels,
                image,
                uniforms,
            });
        }
        let across = size[0] as f32 / small[0] as f32;
        let along = size[1] as f32 / small[1] as f32;
        let nothing = Uniforms {
            tone: [0.0; 4],
            blend: [0.0; 4],
            reach: [0.0; 4],
        };
        passes.push(Pass {
            pipeline: &self.shrink,
            target: &soft[0].view,
            clear: Some([0.0; 3]),
            levels: &self.nothing,
            image: &picture.view,
            uniforms: Uniforms {
                blend: [0.0, 0.0, 4.0, 0.0],
                reach: [across, along, 0.0, 0.0],
                ..nothing
            },
        });
        for (from, to, direction) in [(0, 1, [1.0, 0.0]), (1, 0, [0.0, 1.0])] {
            passes.push(Pass {
                pipeline: &self.blur,
                target: &soft[to].view,
                clear: Some([0.0; 3]),
                levels: &self.nothing,
                image: &soft[from].view,
                uniforms: Uniforms {
                    blend: [0.0, 0.0, 0.0, 1.0],
                    reach: [direction[0], direction[1], quarter / fewer, 0.0],
                    ..nothing
                },
            });
        }

        while self.buffers.len() < passes.len() {
            self.buffers.push(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fractique preview uniforms"),
                size: size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("fractique preview"),
        });
        for (pass, buffer) in passes.iter().zip(&self.buffers) {
            self.queue.write_buffer(buffer, 0, bytemuck::bytes_of(&pass.uniforms));
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fractique preview"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(pass.levels),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(pass.image),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let load = pass.clear.map_or(wgpu::LoadOp::Load, |[r, g, b]| {
                wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 })
            });
            let mut drawing = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fractique preview"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pass.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            drawing.set_pipeline(pass.pipeline);
            drawing.set_bind_group(0, &bind_group, &[]);
            drawing.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_hold_the_levels() {
        assert_eq!(half(0.0), 0);
        assert_eq!(half(-1.0), 0);
        assert_eq!(half(1.0), 0x3c00);
        assert_eq!(half(0.5), 0x3800);
        assert_eq!(half(2.5), 0x4100);
        assert_eq!(half(1e9), 0x7bff);
        assert_eq!(half(2.0f32.powi(-24)), 1);
        assert_eq!(half(2.0f32.powi(-14)), 0x0400);
    }
}
