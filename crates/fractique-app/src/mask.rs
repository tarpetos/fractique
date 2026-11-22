use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

// The corners of a window on Wayland: there the window has no frame of the system, which would round them, so the
// app clears them itself and the window is transparent there.
pub struct Gfx {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

pub fn install(cc: &eframe::CreationContext<'_>) -> bool {
    let Some(state) = &cc.wgpu_render_state else {
        return false;
    };
    if !cc
        .window_handle()
        .is_ok_and(|handle| matches!(handle.as_raw(), RawWindowHandle::Wayland(_)))
    {
        return false;
    }
    let device = &state.device;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fractique corners"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fractique corners"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fractique corners"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fractique corners"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mask.wgsl").into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fractique corners"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    // What is drawn keeps of what was there only the share outside the corner: color and alpha alike, so the
    // picture stays premultiplied.
    let keep = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fractique corners"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: state.target_format,
                blend: Some(wgpu::BlendState {
                    color: keep,
                    alpha: keep,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    state.renderer.write().callback_resources.insert(Gfx {
        pipeline,
        buffer,
        bind_group,
    });
    true
}

pub struct Mask {
    pub radius: f32,
}

impl egui_wgpu::CallbackTrait for Mask {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(gfx) = resources.get::<Gfx>() {
            let shape = [
                screen.size_in_pixels[0] as f32,
                screen.size_in_pixels[1] as f32,
                self.radius * screen.pixels_per_point,
                0.0,
            ];
            queue.write_buffer(&gfx.buffer, 0, bytemuck::cast_slice(&shape));
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(gfx) = resources.get::<Gfx>() {
            render_pass.set_pipeline(&gfx.pipeline);
            render_pass.set_bind_group(0, &gfx.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}
