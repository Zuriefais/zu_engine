use egui_probe::EguiProbe;
use wgpu::{CommandEncoder, Device};

use crate::{
    render_passes::{
        quad_vertex::QuadVertexRenderPass,
        radiance_cascades_passes::radiance_render_compute::RadianceRenderComputePass,
    },
    texture_manager::{TextureManager, textures::TextureType},
};

pub mod radiance_render_compute;

pub struct RadianceCascadesPassesManager {
    compute: RadianceRenderComputePass,
}

impl RadianceCascadesPassesManager {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,

        texture_manager: &mut TextureManager,
    ) -> Self {
        let _ = texture_manager.create_texture(
            "RadianceCascades",
            (width, height),
            device,
            TextureType::Standard,
            1.0,
        );

        let compute = RadianceRenderComputePass::new(device, texture_manager);
        Self { compute }
    }

    pub fn render(
        &mut self,
        render_options: &radiance_render_compute::RadiansOptions,
        encoder: &mut CommandEncoder,
        texture_manager: &mut TextureManager,
    ) {
        self.compute
            .render(encoder, texture_manager, render_options);
    }
}
