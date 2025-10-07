use egui_probe::EguiProbe;
use wgpu::{CommandEncoder, Device, Queue};

use crate::{
    render_passes::{
        quad_vertex::QuadVertexRenderPass,
        radiance_cascades_passes::{
            radiance_render::RadianceRenderPass,
            radiance_render_compute::RadianceRenderComputePass,
            radiance_render_old_pass::RadianceRenderOLDPass,
        },
    },
    texture_manager::{self, TextureManager},
};

pub mod radiance_render;
pub mod radiance_render_compute;
pub mod radiance_render_old_pass;

#[derive(Debug, Clone, Copy, EguiProbe)]
pub enum RadianceMode {
    Fragment,
    Compute,
    FragmentOLD,
}

#[derive(Debug, Clone, Copy, EguiProbe)]
pub struct RadianceCascadesRenderOptions {
    standart_options: radiance_render::RadiansOptions,
    old_options: radiance_render_old_pass::RadiansOptionsOLD,
    compute_options: radiance_render_compute::RadiansOptions,
    radiance_mode: RadianceMode,
}

impl Default for RadianceCascadesRenderOptions {
    fn default() -> Self {
        Self {
            standart_options: Default::default(),
            old_options: Default::default(),
            compute_options: Default::default(),
            radiance_mode: RadianceMode::Compute,
        }
    }
}

pub struct RadianceCascadesPassesManager {
    old_pass: RadianceRenderOLDPass,
    pass: RadianceRenderPass,
    compute: RadianceRenderComputePass,
    width: u32,
    height: u32,
}

impl RadianceCascadesPassesManager {
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
            texture_manager::TextureType::Standart,
            1.0,
        );
        let old_pass = RadianceRenderOLDPass::new(device, quad_render_pass, texture_manager);
        let pass = RadianceRenderPass::new(device, quad_render_pass, texture_manager);
        let compute = RadianceRenderComputePass::new(device, texture_manager);
        Self {
            old_pass,
            pass,
            compute,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height
    }

    pub fn render(
        &mut self,
        render_options: &RadianceCascadesRenderOptions,
        encoder: &mut CommandEncoder,
        texture_manager: &mut TextureManager,
        quad_render_pass: &QuadVertexRenderPass,
    ) {
        match render_options.radiance_mode {
            RadianceMode::Fragment => {
                self.pass.render(
                    encoder,
                    render_options.standart_options,
                    texture_manager,
                    quad_render_pass,
                );
            }
            RadianceMode::Compute => {
                self.compute.render(
                    encoder,
                    texture_manager,
                    render_options.compute_options,
                    self.width,
                    self.height,
                );
            }
            RadianceMode::FragmentOLD => {
                self.old_pass.render(
                    encoder,
                    render_options.old_options,
                    texture_manager,
                    quad_render_pass,
                );
            }
        }
    }
}
