use bytemuck::{Pod, Zeroable, bytes_of};
use wgpu::{
    CommandEncoder, ComputePipelineDescriptor, Device, PushConstantRange, ShaderStages,
    util::RenderEncoder,
};

use crate::texture_manager::{TextureHandle, TextureManager, textures::EngineTexture};

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct JfaConstants {
    pub one_over_size: [f32; 2],
    pub texture_size: [i32; 2],
    pub u_offset: i32,
    pub _pad: f32,
}

pub struct JfaComputePass {
    compute_pipeline: wgpu::ComputePipeline,
}

impl JfaComputePass {
    pub fn new(device: &Device, texture_manager: &mut TextureManager) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/jfa_compute.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Jfa compute pass layout descriptor"),
            bind_group_layouts: &[
                texture_manager.get_compute_storage_rgf16_bind_group_layout(),
                texture_manager.get_compute_storage_mut_rgf16_bind_group_layout(),
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

        JfaComputePass { compute_pipeline }
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        texture_manager: &TextureManager,
        passes: u32,
    ) -> TextureHandle {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("JFA compute pass"),
            timestamp_writes: Default::default(),
        });
        let (width, height) = texture_manager
            .get_texture("JfaTextureF16")
            .unwrap()
            .resolution();
        let wg_x = (width + 7) / 16;
        let wg_y = (height + 7) / 16;
        compute_pass.set_pipeline(&self.compute_pipeline);
        let mut latest_tex = 0;
        for pass_i in 0..passes {
            let u_offset = 2.0_f32.powi((passes - pass_i - 1) as i32) as i32;

            compute_pass.set_push_constants(
                0,
                bytes_of(&JfaConstants {
                    one_over_size: [1.0 / width as f32, 1.0 / height as f32],
                    texture_size: [width as i32, height as i32],
                    u_offset,
                    _pad: 0.0,
                }),
            );

            let (src, dst) = if pass_i % 2 == 0 {
                (
                    texture_manager.get_texture("JfaTextureF16").unwrap(),
                    texture_manager.get_texture("JfaTexture1F16").unwrap(),
                )
            } else {
                (
                    texture_manager.get_texture("JfaTexture1F16").unwrap(),
                    texture_manager.get_texture("JfaTextureF16").unwrap(),
                )
            };
            compute_pass.set_bind_group(0, src.compute_storage_group_rgf16(), &[]);
            compute_pass.set_bind_group(1, dst.compute_storage_mut_group_rgf16(), &[]);
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        latest_tex
    }
}
