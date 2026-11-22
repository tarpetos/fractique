use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};

// The background and the panes of one frame. The buttons of a phone are seven: a sheet over them needs room too.
const SLOTS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub screen: [f32; 4],
    pub rect: [f32; 4],
    pub shape: [f32; 4],
    pub optics: [f32; 4],
    pub view: [f32; 4],
    pub past: [f32; 4],
    pub blend: [f32; 4],
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
    now: (wgpu::TextureView, wgpu::TextureView),
    past: Option<(wgpu::TextureView, wgpu::TextureView)>,
}

// The picture and how it is laid over the window: the picture before it while one fades into the other, and the
// blur over both while a new one is drawn.
#[derive(Clone, Copy)]
pub struct Shown {
    pub view: [f32; 4],
    pub past: [f32; 4],
    pub fresh: f32,
    pub veil: f32,
}

fn one_pixel(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fractique no picture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
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
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
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
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
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
                texture(4),
                texture(5),
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
        // The colour around a picture, as `AROUND` of the shader and the window's clear colour.
        let dark = one_pixel(device, queue, [3, 4, 11, 255]);
        let mut gfx = Self {
            background: pipeline(device, &pipeline_layout, &module, "fs_background", format),
            glass: pipeline(device, &pipeline_layout, &module, "fs_glass", format),
            layout,
            sampler,
            slots,
            now: (dark.clone(), dark),
            past: None,
        };
        gfx.bind(device);
        gfx
    }

    pub fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        flame: (&[u8], u32, u32),
        soft: (&[u8], u32, u32),
    ) -> (wgpu::TextureView, wgpu::TextureView) {
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
        (upload(flame), upload(soft))
    }

    // `fresh`: another picture than the one shown, which stays as the one it fades from.
    pub fn show(&mut self, device: &wgpu::Device, flame: &wgpu::TextureView, soft: &wgpu::TextureView, fresh: bool) {
        let next = (flame.clone(), soft.clone());
        if fresh {
            self.past = Some(std::mem::replace(&mut self.now, next));
        } else {
            self.now = next;
        }
        self.bind(device);
    }

    pub fn forget_past(&mut self, device: &wgpu::Device) {
        if self.past.take().is_some() {
            self.bind(device);
        }
    }

    fn bind(&mut self, device: &wgpu::Device) {
        let (flame, soft) = &self.now;
        let (past_flame, past_soft) = self.past.as_ref().unwrap_or(&self.now);
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
                        resource: wgpu::BindingResource::TextureView(flame),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(soft),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(past_flame),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(past_soft),
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
    Pane {
        rect: egui::Rect,
        optics: Optics,
        opacity: f32,
    },
}

struct Draw {
    slot: usize,
    kind: Kind,
    shown: Shown,
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
            view: self.shown.view,
            past: self.shown.past,
            blend: [self.shown.fresh, self.shown.veil, 1.0, 0.0],
        };
        if let Kind::Pane { rect, optics, opacity } = self.kind {
            uniforms.blend[2] = opacity;
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
    screen: egui::Rect,
    places: Vec<egui::layers::ShapeIdx>,
    panes: Vec<(egui::Rect, Optics, f32)>,
}

impl<'a> Frame<'a> {
    // The picture and the panes are drawn under the widgets but decided after them: the panes stand where the
    // widgets were laid out, and the picture is blurred from the frame of the click that asks for another. Their
    // places in the list of shapes are kept free until then.
    pub fn new(painter: &'a egui::Painter, screen: egui::Rect) -> Self {
        let places = (0..SLOTS).map(|_| painter.add(egui::Shape::Noop)).collect();
        Self {
            painter,
            screen,
            places,
            panes: Vec::new(),
        }
    }

    pub fn pane(&mut self, rect: egui::Rect, optics: Optics, opacity: f32) {
        if opacity > 0.0 && self.panes.len() + 1 < SLOTS {
            self.panes.push((rect, optics, opacity));
        }
    }

    pub fn finish(&mut self, shown: Shown) {
        let background = Draw {
            slot: 0,
            kind: Kind::Background,
            shown,
        };
        self.painter.set(
            self.places[0],
            egui_wgpu::Callback::new_paint_callback(self.screen, background),
        );
        for (index, (rect, optics, opacity)) in self.panes.drain(..).enumerate() {
            let draw = Draw {
                slot: index + 1,
                kind: Kind::Pane { rect, optics, opacity },
                shown,
            };
            let area = rect.expand(SHADOW_REACH);
            self.painter.set(
                self.places[index + 1],
                egui_wgpu::Callback::new_paint_callback(area, draw),
            );
        }
    }
}
