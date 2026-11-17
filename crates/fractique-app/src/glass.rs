use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};

const SLOTS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub screen: [f32; 4],
    pub rect: [f32; 4],
    pub shape: [f32; 4],
    pub optics: [f32; 4],
}

struct Slot {
    buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
}

pub struct Gfx {
    layout: wgpu::BindGroupLayout,
    background: wgpu::RenderPipeline,
    glass: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    slots: Vec<Slot>,
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    fragment: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            entry_point: Some("vs_main"),
            module,
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl Gfx {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fractique glass"),
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
                texture(1),
                texture(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fractique glass"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fractique glass"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fractique glass"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("fractique glass uniforms"),
                    size: size_of::<Uniforms>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                bind_group: None,
            })
            .collect();
        Self {
            background: pipeline(device, &pipeline_layout, &module, "fs_background", format),
            glass: pipeline(device, &pipeline_layout, &module, "fs_glass", format),
            layout,
            sampler,
            slots,
        }
    }

    pub fn set_images(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        flame: (&[u8], u32, u32),
        soft: (&[u8], u32, u32),
    ) {
        let upload = |(bytes, width, height): (&[u8], u32, u32)| {
            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fractique flame"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // Not an sRGB format: egui blends in gamma space, and so do the panes.
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                size,
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };
        let (flame, soft) = (upload(flame), upload(soft));
        for slot in &mut self.slots {
            slot.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fractique glass"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: slot.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&flame),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&soft),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }
    }
}

#[derive(Clone, Copy)]
pub struct Optics {
    pub refraction: f32,
    pub blur: f32,
    pub tint: f32,
    pub rim: f32,
    pub radius: f32,
    pub bezel: f32,
}

#[derive(Clone, Copy)]
enum Kind {
    Background,
    Pane { rect: egui::Rect, optics: Optics },
}

struct Draw {
    slot: usize,
    kind: Kind,
}

const SHADOW_REACH: f32 = 28.0;

impl egui_wgpu::CallbackTrait for Draw {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(gfx) = resources.get::<Gfx>() else {
            return Vec::new();
        };
        let scale = screen.pixels_per_point;
        let mut uniforms = Uniforms {
            screen: [
                screen.size_in_pixels[0] as f32,
                screen.size_in_pixels[1] as f32,
                0.0,
                0.0,
            ],
            rect: [0.0; 4],
            shape: [0.0; 4],
            optics: [0.0; 4],
        };
        if let Kind::Pane { rect, optics } = self.kind {
            uniforms.rect = [
                rect.min.x * scale,
                rect.min.y * scale,
                rect.max.x * scale,
                rect.max.y * scale,
            ];
            uniforms.shape = [optics.radius * scale, optics.bezel * scale, SHADOW_REACH * scale, 0.3];
            uniforms.optics = [optics.refraction * scale, optics.blur, optics.tint, optics.rim];
        }
        queue.write_buffer(&gfx.slots[self.slot].buffer, 0, bytemuck::bytes_of(&uniforms));
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(gfx) = resources.get::<Gfx>() else { return };
        let Some(bind_group) = &gfx.slots[self.slot].bind_group else {
            return;
        };
        pass.set_pipeline(match self.kind {
            Kind::Background => &gfx.background,
            Kind::Pane { .. } => &gfx.glass,
        });
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub struct Frame<'a> {
    painter: &'a egui::Painter,
    next: usize,
}

impl<'a> Frame<'a> {
    pub fn new(painter: &'a egui::Painter, screen: egui::Rect) -> Self {
        painter.add(egui_wgpu::Callback::new_paint_callback(
            screen,
            Draw {
                slot: 0,
                kind: Kind::Background,
            },
        ));
        Self { painter, next: 1 }
    }

    pub fn pane(&mut self, rect: egui::Rect, optics: Optics) {
        if self.next >= SLOTS {
            return;
        }
        let area = rect.expand(SHADOW_REACH);
        let draw = Draw {
            slot: self.next,
            kind: Kind::Pane { rect, optics },
        };
        self.painter.add(egui_wgpu::Callback::new_paint_callback(area, draw));
        self.next += 1;
    }
}
