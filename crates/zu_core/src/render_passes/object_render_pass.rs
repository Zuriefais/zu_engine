// use bytemuck::{Pod, Zeroable};
// use glam::{Vec2, Vec4};
// use wgpu::{
//     BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
//     BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferDescriptor, BufferUsages,
//     CommandEncoder, Device, Queue, ShaderStages, TextureView, VertexBufferLayout,
//     util::{BufferInitDescriptor, DeviceExt},
// };

// use crate::camera::{Camera, CameraUniform};

// #[repr(C)]
// #[derive(Clone, Copy, Pod, Zeroable)]
// struct QuadVertex {
//     position: Vec2,
// }

// impl QuadVertex {
//     fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex, // per-vertex
//             attributes: &[wgpu::VertexAttribute {
//                 offset: 0,
//                 shader_location: 0, // matches @location(0)
//                 format: wgpu::VertexFormat::Float32x2,
//             }],
//         }
//     }
// }

// // a unit quad (two triangles)
// const QUAD_VERTICES: &[QuadVertex] = &[
//     QuadVertex {
//         position: Vec2::new(-0.5, -0.5),
//     },
//     QuadVertex {
//         position: Vec2::new(0.5, -0.5),
//     },
//     QuadVertex {
//         position: Vec2::new(0.5, 0.5),
//     },
//     QuadVertex {
//         position: Vec2::new(-0.5, 0.5),
//     },
// ];

// const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

// #[repr(C)]
// #[derive(Clone, Copy, Pod, Zeroable)]
// struct Instance {
//     position: Vec2,
//     scale: Vec2,
//     color: Vec4,
// }

// impl Instance {
//     fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Instance, // per-instance
//             attributes: &[
//                 // @location(1) position
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//                 // @location(2) scale
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<Vec2>() as wgpu::BufferAddress,
//                     shader_location: 2,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//                 // @location(3) color
//                 wgpu::VertexAttribute {
//                     offset: (mem::size_of::<Vec4>()) as wgpu::BufferAddress,
//                     shader_location: 3,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//             ],
//         }
//     }
// }

// pub struct ObjectRenderPass {
//     render_pipeline: wgpu::RenderPipeline,
//     camera_bind_group: BindGroup,
//     camera_buffer: Buffer,
//     vertex_buffer: Buffer,
//     index_buffer: Buffer,
//     index_count: u32,
// }

// impl ObjectRenderPass {
//     pub fn new(device: &Device, config: &wgpu::SurfaceConfiguration, camera: &Camera) -> Self {
//         let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/obj_draw.wgsl"));

//         let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
//             label: Some(&"Camera"),
//             contents: bytemuck::bytes_of(&camera.get_camera_uninform()),
//             usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
//         });

//         let camera_bind_group_layout =
//             device.create_bind_group_layout(&BindGroupLayoutDescriptor {
//                 label: Some("Camera"),
//                 entries: &[BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: ShaderStages::VERTEX,
//                     ty: BindingType::Buffer {
//                         ty: wgpu::BufferBindingType::Uniform,
//                         has_dynamic_offset: false,
//                         min_binding_size: None,
//                     },
//                     count: None,
//                 }],
//             });

//         let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
//             label: Some("Camera"),
//             layout: &camera_bind_group_layout,
//             entries: &[BindGroupEntry {
//                 binding: 0,
//                 resource: camera_buffer.as_entire_binding(),
//             }],
//         });

//         let render_pipeline_layout =
//             device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//                 label: Some("Render Pipeline Layout"),
//                 bind_group_layouts: &[&camera_bind_group_layout],
//                 push_constant_ranges: &[],
//             });

//         let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//             label: Some("Render Pipeline"),
//             layout: Some(&render_pipeline_layout),
//             vertex: wgpu::VertexState {
//                 module: &shader,
//                 entry_point: Some("vs_main"),                     // 1.
//                 buffers: &[QuadVertex::desc(), Instance::desc()], // 2.
//                 compilation_options: wgpu::PipelineCompilationOptions::default(),
//             },
//             fragment: Some(wgpu::FragmentState {
//                 // 3.
//                 module: &shader,
//                 entry_point: Some("fs_main"),
//                 targets: &[Some(wgpu::ColorTargetState {
//                     // 4.
//                     format: config.format,
//                     blend: Some(wgpu::BlendState::REPLACE),
//                     write_mask: wgpu::ColorWrites::ALL,
//                 })],
//                 compilation_options: wgpu::PipelineCompilationOptions::default(),
//             }),
//             primitive: wgpu::PrimitiveState {
//                 topology: wgpu::PrimitiveTopology::TriangleList, // 1.
//                 strip_index_format: None,
//                 front_face: wgpu::FrontFace::Ccw, // 2.
//                 cull_mode: Some(wgpu::Face::Back),
//                 // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
//                 polygon_mode: wgpu::PolygonMode::Fill,
//                 // Requires Features::DEPTH_CLIP_CONTROL
//                 unclipped_depth: false,
//                 // Requires Features::CONSERVATIVE_RASTERIZATION
//                 conservative: false,
//             },
//             depth_stencil: None, // 1.
//             multisample: wgpu::MultisampleState {
//                 count: 1,                         // 2.
//                 mask: !0,                         // 3.
//                 alpha_to_coverage_enabled: false, // 4.
//             },
//             multiview: None, // 5.
//             cache: None,     // 6.
//         });

//         let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Quad Vertex Buffer"),
//             contents: bytemuck::cast_slice(QUAD_VERTICES),
//             usage: wgpu::BufferUsages::VERTEX,
//         });
//         let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Quad Index Buffer"),
//             contents: bytemuck::cast_slice(QUAD_INDICES),
//             usage: wgpu::BufferUsages::INDEX,
//         });
//         let index_count = QUAD_INDICES.len() as u32;

//         ObjectRenderPass {
//             render_pipeline,
//             camera_bind_group,
//             index_buffer,
//             index_count,
//             vertex_buffer,
//             camera_buffer,
//         }
//     }

//     pub fn render(
//         &mut self,
//         encoder: &mut CommandEncoder,
//         device: &Device,
//         queue: &Queue,
//         view: &TextureView,
//         camera: &Camera,
//     ) {
//         let instances = vec![
//             Instance {
//                 position: Vec2::ZERO,
//                 scale: Vec2::new(1.0, 1.0),
//                 color: Vec4::new(1.0, 1.0, 0.0, 1.0),
//             },
//             Instance {
//                 position: Vec2::new(0.0, 1.0),
//                 scale: Vec2::new(1.0, 1.0),
//                 color: Vec4::new(1.0, 0.0, 1.0, 1.0),
//             },
//         ];

//         // Per frame: upload instance buffer
//         let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Instance Buffer"),
//             contents: bytemuck::cast_slice(&instances), // Vec<Instance>
//             usage: wgpu::BufferUsages::VERTEX,
//         });

//         queue.write_buffer(
//             &self.camera_buffer,
//             0,
//             bytemuck::bytes_of(&camera.get_camera_uninform()),
//         );

//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("Render Pass"),
//             color_attachments: &[
//                 // This is what @location(0) in the fragment shader targets
//                 Some(wgpu::RenderPassColorAttachment {
//                     view,
//                     resolve_target: None,
//                     ops: wgpu::Operations {
//                         load: wgpu::LoadOp::Clear(wgpu::Color {
//                             r: 0.1,
//                             g: 0.2,
//                             b: 0.3,
//                             a: 1.0,
//                         }),
//                         store: wgpu::StoreOp::Store,
//                     },
//                 }),
//             ],
//             depth_stencil_attachment: None,
//             timestamp_writes: Default::default(),
//             occlusion_query_set: Default::default(),
//         });

//         render_pass.set_pipeline(&self.render_pipeline); // 2.
//         render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
//         render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
//         render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//         render_pass.draw_indexed(0..self.index_count, 0, 0..instances.len() as u32);
//     }
// }
