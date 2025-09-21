use egui_probe::EguiProbe;
use glam::Vec2;
use wgpu::{CommandEncoder, Device, Queue, TextureView};

use crate::{
    render_passes::{
        quad_vertex::QuadVertexRenderPass,
        radiance_render_pass::{RadianceRenderPass, RadiansOptions},
    },
    scene_texture::{self, SceneTexture},
};

#[derive(Debug, Default, Copy, Clone, EguiProbe)]
pub struct RenderOptions {
    radians_options: RadiansOptions,
}

pub struct RenderPassManager {
    radiance_pass: RadianceRenderPass,
    quad_render_pass: QuadVertexRenderPass,
    render_options: RenderOptions,
    scene_texture: SceneTexture,
}

impl RenderPassManager {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> RenderPassManager {
        let quad_render_pass = QuadVertexRenderPass::new(device);
        let scene_texture = SceneTexture::new(width, height, device);
        let radiance_pass = RadianceRenderPass::new(
            device,
            config,
            width,
            height,
            &quad_render_pass,
            &scene_texture,
        );
        Self {
            radiance_pass,
            quad_render_pass,
            render_options: Default::default(),
            scene_texture,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        self.scene_texture.resize(width, height, device);
        self.radiance_pass
            .resize(width, height, device, queue, &self.scene_texture);
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        device: &Device,
        queue: &Queue,
    ) {
        self.radiance_pass.render(
            encoder,
            device,
            queue,
            view,
            self.render_options.radians_options,
            &self.quad_render_pass,
        );
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
        self.scene_texture
            .paint(pos, color, brush_radius, width, height, queue);
    }
}
