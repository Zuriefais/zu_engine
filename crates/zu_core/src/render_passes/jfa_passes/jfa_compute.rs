use bytemuck::{Pod, Zeroable, bytes_of};
use glam::Vec2;
use log::info;
use wgpu::{
    BindGroup, Buffer, BufferUsages, CommandEncoder, ComputePipelineDescriptor, Device,
    PushConstantRange, Queue, ShaderStages, TextureView,
    util::{BufferInitDescriptor, DeviceExt, RenderEncoder},
};

use crate::{
    render_passes::quad_vertex::QuadVertexRenderPass,
    texture_manager::{self, TextureManager},
    vertex_state_for_quad,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct JfaConstants {
    pub one_over_size: [f32; 2],
    pub texture_size: [f32; 2],
    pub u_offset: f32, // <-- new: jump distance for this pass
    pub _pad: f32,     // keep alignment (32 bytes total)
}

pub struct JfaComputePass {
    compute_pipeline: wgpu::ComputePipeline,

    width: u32,
    height: u32,
}

impl JfaComputePass {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        texture_manager: &mut TextureManager,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/jfa_compute.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&"Jfa compute pass layout descriptor"),
            bind_group_layouts: &[
                texture_manager.get_compute_bind_group_layout(),
                texture_manager.get_compute_mut_bind_group_layout(),
            ],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..std::mem::size_of::<JfaConstants>() as u32,
            }],
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Jfa compute pass"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        texture_manager.create_texture(
            "JfaTexture",
            (width, height),
            device,
            texture_manager::TextureType::Standart,
            1.0,
        );
        texture_manager.create_texture(
            "JfaTexture1",
            (width, height),
            device,
            texture_manager::TextureType::Standart,
            1.0,
        );
        JfaComputePass {
            compute_pipeline,

            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        passes: u32,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("JFA compute pass"),
            timestamp_writes: Default::default(),
        });
        let wg_x = (self.width + 7) / 16;
        let wg_y = (self.height + 7) / 16;
        compute_pass.set_pipeline(&self.compute_pipeline);
        for pass_i in 0..passes {
            let u_offset = 2.0_f32.powi((passes - pass_i - 1) as i32);

            compute_pass.set_push_constants(
                0,
                bytes_of(&JfaConstants {
                    one_over_size: [1.0 / self.width as f32, 1.0 / self.height as f32],
                    texture_size: [self.width as f32, self.height as f32],
                    u_offset,
                    _pad: 0.0,
                }),
            );

            let (src, dst) = if pass_i % 2 == 0 {
                (
                    texture_manager.get_texture("JfaTexture").unwrap(),
                    texture_manager.get_texture("JfaTexture1").unwrap(),
                )
            } else {
                (
                    texture_manager.get_texture("JfaTexture1").unwrap(),
                    texture_manager.get_texture("JfaTexture").unwrap(),
                )
            };

            compute_pass.set_bind_group(0, src.compute_bind_group(), &[]);
            compute_pass.set_bind_group(1, dst.compute_mut_group(), &[]);
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }
}
