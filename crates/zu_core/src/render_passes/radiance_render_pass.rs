use bytemuck::{Pod, Zeroable};
use egui_probe::EguiProbe;
use glam::{Vec2, Vec4};
use log::info;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferDescriptor, BufferUsages,
    CommandEncoder, Device, Queue, ShaderModuleDescriptor, ShaderStages, Texture, TextureView,
    VertexBufferLayout,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{
    camera::{Camera, CameraUniform},
    render_passes::quad_vertex::QuadVertexRenderPass,
    scene_texture::SceneTexture,
    vertex_state_for_quad,
};

fn create_buffers(
    device: &Device,
    width: u32,
    height: u32,
) -> (Buffer, Buffer, Buffer, Buffer, Buffer) {
    let ray_count_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some(&"Ray Count Buffer"),
        contents: bytemuck::bytes_of(&8i32), // Use i32 to match WGSL
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let size_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some(&"Size Buffer"),
        contents: bytemuck::bytes_of(&Vec2::new(width as f32, height as f32)),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let accum_radiance_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some(&"Accum Radiance Buffer"),
        contents: bytemuck::bytes_of(&1i32), // Use i32 to match WGSL
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let max_steps_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some(&"Max Steps Buffer"),
        contents: bytemuck::bytes_of(&128i32), // Use i32 to match WGSL
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let enable_noise_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some(&"Enable noise Buffer"),
        contents: bytemuck::bytes_of(&1i32), // Use i32 to match WGSL
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    (
        ray_count_buffer,
        size_buffer,
        accum_radiance_buffer,
        max_steps_buffer,
        enable_noise_buffer,
    )
}

pub struct RadianceRenderPass {
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group: BindGroup,
    ray_count_buffer: Buffer,
    size_buffer: Buffer,
    accum_radiance_buffer: Buffer,
    enable_noise_buffer: Buffer,
    max_steps_buffer: Buffer,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl RadianceRenderPass {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
        quad_render_pass: &QuadVertexRenderPass,
        scene_texture: &SceneTexture,
    ) -> Self {
        let radiance_shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/radiance_cascades.wgsl"));

        let (
            ray_count_buffer,
            size_buffer,
            accum_radiance_buffer,
            max_steps_buffer,
            enable_noise_buffer,
        ) = create_buffers(device, width, height);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Radiance Cascades Bind Group Layout"),
                entries: &[
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Texture
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
                    // ray_count
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
                    // size
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
                    // accum_radiance
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
                    // max_steps
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // enable noise
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
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

        let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Radiance Cascades Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(scene_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ray_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: accum_radiance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: max_steps_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: enable_noise_buffer.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: vertex_state_for_quad!(quad_render_pass),
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &radiance_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,     // 6.
        });

        RadianceRenderPass {
            render_pipeline,

            texture_bind_group,

            ray_count_buffer,
            size_buffer,
            accum_radiance_buffer,
            max_steps_buffer,
            sampler,
            bind_group_layout: texture_bind_group_layout,
            enable_noise_buffer,
        }
    }

    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
        device: &Device,
        queue: &Queue,
        scene_texture: &SceneTexture,
    ) {
        queue.write_buffer(
            &self.size_buffer,
            0,
            bytemuck::bytes_of(&Vec2::new(width as f32, height as f32)),
        );

        self.texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Radiance Cascades Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(scene_texture.view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.ray_count_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.accum_radiance_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: self.max_steps_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: self.enable_noise_buffer.as_entire_binding(),
                },
            ],
        });
        info!("Texture resized");
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        options: RadiansOptions,
        quad_render_pass: &QuadVertexRenderPass,
    ) {
        queue.write_buffer(
            &self.ray_count_buffer,
            0,
            bytemuck::bytes_of(&(options.ray_count as i32)),
        );
        queue.write_buffer(
            &self.accum_radiance_buffer,
            0,
            bytemuck::bytes_of(&(options.accum_radiance as i32)), // Convert bool to i32 for WGSL
        );
        queue.write_buffer(
            &self.enable_noise_buffer,
            0,
            bytemuck::bytes_of(&(options.enable_noise as i32)), // Convert bool to i32 for WGSL
        );
        queue.write_buffer(
            &self.max_steps_buffer,
            0,
            bytemuck::bytes_of(&(options.max_steps as i32)),
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[
                // This is what @location(0) in the fragment shader targets
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: Default::default(),
            occlusion_query_set: Default::default(),
        });

        render_pass.set_pipeline(&self.render_pipeline); // 2.
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        quad_render_pass.render(&mut render_pass);
    }
}

#[derive(Debug, Clone, Copy, EguiProbe)]
pub struct RadiansOptions {
    ray_count: u32,
    accum_radiance: bool,
    max_steps: u32,
    enable_noise: bool,
}

impl Default for RadiansOptions {
    fn default() -> Self {
        Self {
            ray_count: 8,
            accum_radiance: true,
            max_steps: 128,
            enable_noise: true,
        }
    }
}
