use egui_probe::EguiProbe;
use glam::Vec2;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    CommandEncoder, Device, Queue, Sampler, Texture, TextureView,
};

use crate::{
    render_passes::{
        distant_field_pass::{self, DistantFieldPass},
        jfa_passes::{
            JfaPassesManager, JfaRenderOptions, jfa_compute::JfaComputePass,
            jfa_compute_pass_one_shot::JfaComputeOneShotPass, jfa_pass::JfaRenderPass,
        },
        quad_vertex::QuadVertexRenderPass,
        radiance_cascades_passes::{RadianceCascadesPassesManager, RadianceCascadesRenderOptions},
        seed_pass::{self, SeedRenderPass},
        show_pass::{self, ShowRenderPass},
    },
    texture_manager::{
        self, TextureManager,
        textures::{EngineTexture, ManagedTexture, TextureType},
    },
};

#[derive(Debug, Clone, EguiProbe)]
pub struct RenderOptions {
    radiance_options: RadianceCascadesRenderOptions,
    jfa_options: JfaRenderOptions,
    show: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            radiance_options: Default::default(),
            jfa_options: Default::default(),
            show: "RadianceCascades".into(),
        }
    }
}

pub struct RenderPassManager {
    distant_field_pass: DistantFieldPass,
    show_pass: ShowRenderPass,
    quad_render_pass: QuadVertexRenderPass,
    render_options: RenderOptions,
    texture_manager: TextureManager,
    radiance_passes_manager: RadianceCascadesPassesManager,
    jfa_passes_manager: JfaPassesManager,
    width: u32,
    height: u32,
}

impl RenderPassManager {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> RenderPassManager {
        let mut texture_manager = TextureManager::new(device);
        texture_manager.create_texture(
            "SceneTexture",
            (width, height),
            device,
            TextureType::SceneTexture,
            1.0,
        );
        let quad_render_pass = QuadVertexRenderPass::new(device);
        let jfa_passes_manager = JfaPassesManager::new(
            device,
            width,
            height,
            &quad_render_pass,
            &mut texture_manager,
        );
        let show_pass = ShowRenderPass::new(device, config, &quad_render_pass);
        let distant_field_pass = DistantFieldPass::new(
            device,
            &quad_render_pass,
            width,
            height,
            &mut texture_manager,
        );
        let radiance_passes_manager = RadianceCascadesPassesManager::new(
            device,
            width,
            height,
            &quad_render_pass,
            &mut texture_manager,
        );

        Self {
            quad_render_pass,
            render_options: Default::default(),
            show_pass,
            distant_field_pass,
            texture_manager,
            radiance_passes_manager,
            jfa_passes_manager,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        puffin::profile_function!();
        self.texture_manager.resize(device, (width, height));
        self.jfa_passes_manager.resize(device, width, height);
        self.radiance_passes_manager.resize(width, height);
        self.width = width;
        self.height = height;
    }

    pub fn render(&mut self, view: &TextureView, encoder: &mut CommandEncoder, device: &Device) {
        puffin::profile_function!();
        self.jfa_passes_manager.render(
            &self.render_options.jfa_options,
            encoder,
            &mut self.texture_manager,
            &self.quad_render_pass,
            self.width,
            self.height,
        );
        self.distant_field_pass.render(
            encoder,
            device,
            &self.texture_manager,
            &self.quad_render_pass,
        );
        self.radiance_passes_manager.render(
            &self.render_options.radiance_options,
            encoder,
            &mut self.texture_manager,
            &self.quad_render_pass,
        );
        if let Some(texture) = self.texture_manager.get_texture(&self.render_options.show) {
            self.show_pass
                .render(encoder, texture.bind_group(), view, &self.quad_render_pass);
        }
    }

    pub fn get_options(&mut self) -> &mut RenderOptions {
        &mut self.render_options
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
        if let Some(ManagedTexture::SceneTexture(texture)) =
            self.texture_manager.get_texture_mut("SceneTexture")
        {
            texture.paint(pos, color, brush_radius, width, height, queue);
        }
    }
}
