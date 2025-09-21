use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec4};
use log::info;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferDescriptor, BufferUsages,
    CommandEncoder, Device, Queue, ShaderModuleDescriptor, ShaderStages, Texture, TextureView,
    VertexBufferLayout,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::camera::{Camera, CameraUniform};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadVertex {
    position: Vec2,
}

impl QuadVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex, // per-vertex
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0, // matches @location(0)
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

// a unit quad (two triangles)
const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: Vec2::new(-0.5, -0.5),
    },
    QuadVertex {
        position: Vec2::new(0.5, -0.5),
    },
    QuadVertex {
        position: Vec2::new(0.5, 0.5),
    },
    QuadVertex {
        position: Vec2::new(-0.5, 0.5),
    },
];

const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

fn create_texture(width: u32, height: u32, device: &Device) -> (Texture, TextureView) {
    let texture_size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("diffuse_texture"),
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, texture_view)
}

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

pub struct FragmentRenderPass {
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group: BindGroup,
    texture: Texture,
    texture_view: TextureView,
    texture_data: Vec<u8>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    ray_count_buffer: Buffer,
    size_buffer: Buffer,
    accum_radiance_buffer: Buffer,
    enable_noise_buffer: Buffer,
    max_steps_buffer: Buffer,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl FragmentRenderPass {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> Self {
        let radiance_shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/radiance_cascades.wgsl"));
        let quad_vertex_shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/quad_vertex.wgsl"));

        let (texture, texture_view) = create_texture(width, height, device);
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
                    resource: wgpu::BindingResource::TextureView(&texture_view),
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
            vertex: wgpu::VertexState {
                module: &quad_vertex_shader,
                entry_point: Some("vs_main"),   // 1.
                buffers: &[QuadVertex::desc()], // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Index Buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let index_count = QUAD_INDICES.len() as u32;

        FragmentRenderPass {
            render_pipeline,
            texture,
            texture_bind_group,
            texture_view,
            index_buffer,
            index_count,
            vertex_buffer,
            texture_data: vec![],
            ray_count_buffer,
            size_buffer,
            accum_radiance_buffer,
            max_steps_buffer,
            sampler,
            bind_group_layout: texture_bind_group_layout,
            enable_noise_buffer,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        let (texture, texture_view) = create_texture(width, height, device);
        self.texture = texture;
        self.texture_view = texture_view;

        queue.write_buffer(
            &self.size_buffer,
            0,
            bytemuck::bytes_of(&Vec2::new(width as f32, height as f32)),
        );

        let pixel_count = (width * height) as usize;
        let flat_rgba: Vec<u8> = vec![[0u8, 0u8, 0u8, 0u8]; pixel_count]
            .into_iter()
            .flatten()
            .collect();

        // compute padded bytes per row & build a padded buffer
        let unpadded_bytes_per_row = width * 4;
        let bytes_per_row = padded_bytes_per_row(unpadded_bytes_per_row) as usize;
        let mut padded: Vec<u8> = vec![0; bytes_per_row * height as usize];

        // copy each source row into the padded row
        for row in 0..height as usize {
            let src_start = row * (width as usize) * 4;
            let src_end = src_start + (width as usize) * 4;
            let dst_start = row * bytes_per_row;
            padded[dst_start..dst_start + (width as usize) * 4]
                .copy_from_slice(&flat_rgba[src_start..src_end]);
        }

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        self.texture_data = flat_rgba;
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
                    resource: BindingResource::TextureView(&self.texture_view),
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
        ray_count: u32,
        accum_radiance: bool,
        max_steps: u32,
        enable_noise: bool,
    ) {
        queue.write_buffer(
            &self.ray_count_buffer,
            0,
            bytemuck::bytes_of(&(ray_count as i32)),
        );
        queue.write_buffer(
            &self.accum_radiance_buffer,
            0,
            bytemuck::bytes_of(&(accum_radiance as i32)), // Convert bool to i32 for WGSL
        );
        queue.write_buffer(
            &self.enable_noise_buffer,
            0,
            bytemuck::bytes_of(&(enable_noise as i32)), // Convert bool to i32 for WGSL
        );
        queue.write_buffer(
            &self.max_steps_buffer,
            0,
            bytemuck::bytes_of(&(max_steps as i32)),
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
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    pub fn paint(
        &mut self,
        pos: Vec2,
        color: [f32; 4],
        brush_radius: u32,
        width: u32,
        height: u32,
        queue: &Queue,
    ) {
        // Convert color from f32 [0.0, 1.0] to u8 [0, 255]
        let rgba = [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            255,
        ];

        let center_x = pos.x as i32;
        let center_y = (height as f32 - pos.y) as i32;
        let radius_sq = (brush_radius as i32).pow(2);

        info!("Painting at {}, {}, color: {:?}", center_x, center_y, rgba);

        let min_x = (center_x - brush_radius as i32).max(0) as u32;
        let max_x = (center_x + brush_radius as i32).min((width - 1) as i32) as u32;
        let min_y = (center_y - brush_radius as i32).max(0) as u32;
        let max_y = (center_y + brush_radius as i32).min((height - 1) as i32) as u32;

        if min_x > max_x || min_y > max_y {
            return;
        }

        let rect_width = max_x - min_x + 1;
        let rect_height = max_y - min_y + 1;

        let bytes_per_row = padded_bytes_per_row(rect_width * 4);
        let mut patch_data = vec![0u8; (bytes_per_row * rect_height) as usize];

        for y_in_rect in 0..rect_height {
            for x_in_rect in 0..rect_width {
                let tex_x = min_x + x_in_rect;
                let tex_y = min_y + y_in_rect;

                let dx = tex_x as i32 - center_x;
                let dy = tex_y as i32 - center_y;

                let patch_idx_start = (y_in_rect * bytes_per_row + x_in_rect * 4) as usize;

                if dx * dx + dy * dy <= radius_sq {
                    patch_data[patch_idx_start..patch_idx_start + 4].copy_from_slice(&rgba);
                    let cpu_idx = ((tex_y * width) + tex_x) as usize * 4;
                    self.texture_data[cpu_idx..cpu_idx + 4].copy_from_slice(&rgba);
                } else {
                    // If outside, use the existing color from our CPU-side copy
                    let cpu_idx = ((tex_y * width) + tex_x) as usize * 4;
                    let old_color = &self.texture_data[cpu_idx..cpu_idx + 4];
                    patch_data[patch_idx_start..patch_idx_start + 4].copy_from_slice(old_color);
                }
            }
        }

        // 4. Write the entire patch to the GPU texture in a single call
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: min_x,
                    y: min_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &patch_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rect_height),
            },
            wgpu::Extent3d {
                width: rect_width,
                height: rect_height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn padded_bytes_per_row(unpadded_row_bytes: u32) -> u32 {
    // WebGPU requires bytes_per_row be a multiple of 256
    const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    ((unpadded_row_bytes + ALIGN - 1) / ALIGN) * ALIGN
}
