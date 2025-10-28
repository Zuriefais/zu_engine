pub mod jfa_compute;
use egui_probe::EguiProbe;
use wgpu::{CommandEncoder, Device};

use crate::{
    render_passes::{
        jfa_passes::jfa_compute::JfaComputePass, quad_vertex::QuadVertexRenderPass,
        seed_pass::SeedRenderPass,
    },
    texture_manager::{TextureManager, textures::TextureType},
};

#[derive(Debug, Clone, Copy, EguiProbe)]
pub struct JfaRenderOptions {
    passes: u32,
}

impl Default for JfaRenderOptions {
    fn default() -> Self {
        Self { passes: 9 }
    }
}

pub struct JfaPassesManager {
    compute: JfaComputePass,
    seed_pass: SeedRenderPass,
    texture_f16: usize,
}

impl JfaPassesManager {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        quad_render_pass: &QuadVertexRenderPass,
        texture_manager: &mut TextureManager,
    ) -> Self {
        let _ = texture_manager.create_texture(
            "RadianceCascades",
            (width, height),
            device,
            TextureType::Standard,
            1.0,
        );
        let (texture3, texture4) = (
            texture_manager.create_texture(
                "JfaTextureF16",
                (width, height),
                device,
                TextureType::StandardRGF16,
                1.0,
            ),
            texture_manager.create_texture(
                "JfaTexture1F16",
                (width, height),
                device,
                TextureType::StandardRGF16,
                1.0,
            ),
        );
        Self {
            compute: JfaComputePass::new(device, texture_manager),
            seed_pass: SeedRenderPass::new(device, texture_manager, quad_render_pass),
            texture_f16: texture3,
        }
    }

    pub fn render(
        &mut self,
        options: &JfaRenderOptions,
        encoder: &mut CommandEncoder,
        texture_manager: &mut TextureManager,
        quad_render_pass: &QuadVertexRenderPass,
    ) -> usize {
        self.seed_pass
            .render(encoder, texture_manager, quad_render_pass, true);
        self.compute
            .render(encoder, texture_manager, options.passes);
        self.texture_f16
    }
}
