use glam::Vec2;
use log::info;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer, BufferUsages,
    CommandEncoder, Device, Queue, TextureView,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::render_passes::quad_vertex::QuadVertexRenderPass;
use crate::vertex_state_for_quad;

fn create_buffers(device: &Device, width: u32, height: u32) -> (Buffer, Buffer) {
    let one_over_size = Vec2::new(1.0 / width as f32, 1.0 / height as f32);
    let one_over_size_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("One Over Size Buffer"),
        contents: bytemuck::bytes_of(&one_over_size),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let u_offset_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Offset Buffer"),
        contents: bytemuck::bytes_of(&1.0f32), // начальное смещение
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    (one_over_size_buffer, u_offset_buffer)
}

pub struct JfaRenderPass {
    render_pipeline: wgpu::RenderPipeline,
    one_over_size_buffer: Buffer,
    u_offset_buffer: Buffer,
    skip_buffer: Buffer,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl JfaRenderPass {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
        quad_render_pass: &QuadVertexRenderPass,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/jfa.wgsl"));

        let (one_over_size_buffer, u_offset_buffer) = create_buffers(device, width, height);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let skip_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Skip Buffer"),
            contents: bytemuck::bytes_of(&0.0f32), // изначально skip = 0.0 (false)
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("JFA Bind Group Layout"),
            entries: &[
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // one_over_size
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // u_offset
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("JFA Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("JFA Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state_for_quad!(quad_render_pass),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        JfaRenderPass {
            render_pipeline,
            one_over_size_buffer,
            u_offset_buffer,
            sampler,
            skip_buffer,
            bind_group_layout,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, queue: &Queue) {
        let one_over_size = Vec2::new(1.0 / width as f32, 1.0 / height as f32);

        queue.write_buffer(
            &self.one_over_size_buffer,
            0,
            bytemuck::bytes_of(&one_over_size),
        );
        info!("JFA resized");
    }

    pub fn set_skip(&self, queue: &Queue, skip: bool) {
        let val: f32 = if skip { 1.0 } else { 0.0 };
        queue.write_buffer(&self.skip_buffer, 0, bytemuck::bytes_of(&val));
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &Device,
        queue: &Queue,
        input_texture: &TextureView,
        output_view: &TextureView,
        offset: f32,
        quad_render_pass: &QuadVertexRenderPass,
    ) {
        queue.write_buffer(&self.u_offset_buffer, 0, bytemuck::bytes_of(&offset));

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("JFA Bind Group (per-frame)"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(input_texture),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.one_over_size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.u_offset_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.skip_buffer.as_entire_binding(),
                },
            ],
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("JFA Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: Default::default(),
            occlusion_query_set: Default::default(),
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        quad_render_pass.render(&mut render_pass);
    }
}
