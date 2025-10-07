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
            jfa_compute::JfaComputePass, jfa_compute_pass_one_shot::JfaComputeOneShotPass,
            jfa_pass::JfaRenderPass,
        },
        quad_vertex::QuadVertexRenderPass,
        radiance_cascades_passes::{RadianceCascadesPassesManager, RadianceCascadesRenderOptions},
        seed_pass::{self, SeedRenderPass},
        show_pass::{self, ShowRenderPass},
    },
    texture_manager::{self, ManagedTexture, TextureManager},
};

#[derive(Debug, Clone, EguiProbe)]
enum JfaMode {
    Compute,
    ComputeOneShot,
    Fragment,
}

#[derive(Debug, Clone, EguiProbe)]
pub struct RenderOptions {
    radiance_options: RadianceCascadesRenderOptions,
    jfa_passes_count: u32,
    jfa_mode: JfaMode,
    show: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            radiance_options: Default::default(),
            jfa_passes_count: 9,
            show: "RadianceCascades".into(),
            jfa_mode: JfaMode::Compute,
        }
    }
}

pub struct RenderPassManager {
    jfa_pass: JfaRenderPass,
    jfa_compute_pass: JfaComputePass,
    jfa_compute_one_shot_pass: JfaComputeOneShotPass,
    seed_pass: SeedRenderPass,
    distant_field_pass: DistantFieldPass,
    show_pass: ShowRenderPass,
    quad_render_pass: QuadVertexRenderPass,
    render_options: RenderOptions,
    texture_manager: TextureManager,
    radiance_passes_manager: RadianceCascadesPassesManager,
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
            texture_manager::TextureType::SceneTexture,
            1.0,
        );
        let quad_render_pass = QuadVertexRenderPass::new(device);
        let jfa_pass = JfaRenderPass::new(
            device,
            width,
            height,
            &quad_render_pass,
            &mut texture_manager,
        );
        let jfa_compute_pass = JfaComputePass::new(device, width, height, &mut texture_manager);
        let jfa_compute_one_shot_pass =
            JfaComputeOneShotPass::new(device, width, height, &mut texture_manager);
        let seed_pass = SeedRenderPass::new(device, &texture_manager, &quad_render_pass);
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
            jfa_pass,
            quad_render_pass,
            render_options: Default::default(),
            seed_pass,
            show_pass,
            distant_field_pass,
            texture_manager,
            jfa_compute_pass,
            jfa_compute_one_shot_pass,
            radiance_passes_manager,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        puffin::profile_function!();
        self.texture_manager.resize(device, (width, height));
        self.jfa_pass.resize(width, height);
        self.jfa_compute_pass.resize(width, height);
        self.jfa_compute_one_shot_pass.resize(width, height);
        self.radiance_passes_manager.resize(width, height);
    }

    pub fn render(&mut self, view: &TextureView, encoder: &mut CommandEncoder, device: &Device) {
        puffin::profile_function!();
        match self.render_options.jfa_mode {
            JfaMode::Compute => {
                self.seed_pass
                    .render(encoder, &self.texture_manager, &self.quad_render_pass);
                self.jfa_compute_pass.render(
                    encoder,
                    &self.texture_manager,
                    self.render_options.jfa_passes_count,
                );
            }
            JfaMode::ComputeOneShot => {
                self.jfa_compute_one_shot_pass.render(
                    encoder,
                    &self.texture_manager,
                    self.render_options.jfa_passes_count,
                );
            }
            JfaMode::Fragment => {
                self.seed_pass
                    .render(encoder, &self.texture_manager, &self.quad_render_pass);
                self.jfa_pass.multi_render(
                    encoder,
                    &self.quad_render_pass,
                    &self.texture_manager,
                    self.render_options.jfa_passes_count as i32,
                );
            }
        }
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
