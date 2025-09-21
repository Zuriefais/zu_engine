use egui_probe::EguiProbe;
use glam::Vec2;
use wgpu::{CommandEncoder, Device, Queue, Texture, TextureView};

use crate::{
    render_passes::{
        distant_field_pass::{self, DistantFieldPass},
        jfa_pass::{self, JfaRenderPass},
        quad_vertex::QuadVertexRenderPass,
        radiance_render_old_pass::{RadianceRenderOLDPass, RadiansOptions},
        seed_pass::{self, SeedRenderPass},
        show_pass::{self, ShowRenderPass},
    },
    scene_texture::SceneTexture,
};

fn create_texture(device: &Device, width: u32, height: u32) -> TextureView {
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
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("diffuse_texture"),
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    texture_view
}

#[derive(Debug, Copy, Clone, EguiProbe)]
enum TextureOption {
    SceneTexture,
    Texture1,
    Texture2,
    DistanceField,
    RadiansCascadesOld,
}

#[derive(Debug, Copy, Clone, EguiProbe)]
pub struct RenderOptions {
    radians_options: RadiansOptions,
    radians_old_enabled: bool,
    jfa_passes_count: u32,
    show: TextureOption,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            radians_options: Default::default(),
            jfa_passes_count: 9,
            radians_old_enabled: true,
            show: TextureOption::RadiansCascadesOld,
        }
    }
}

pub struct RenderPassManager {
    jfa_pass: JfaRenderPass,
    seed_pass: SeedRenderPass,
    radiance_old_pass: RadianceRenderOLDPass,
    distant_field_pass: DistantFieldPass,
    show_pass: ShowRenderPass,
    quad_render_pass: QuadVertexRenderPass,
    render_options: RenderOptions,
    scene_texture: SceneTexture,
    texture1: TextureView,
    texture2: TextureView,
    distant_field_texture: TextureView,

    radians_cascades_old: TextureView,
}

impl RenderPassManager {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> RenderPassManager {
        let quad_render_pass = QuadVertexRenderPass::new(device);
        let jfa_pass = JfaRenderPass::new(device, config, width, height, &quad_render_pass);
        let seed_pass = SeedRenderPass::new(device, config, width, height, &quad_render_pass);
        let show_pass = ShowRenderPass::new(device, config, width, height, &quad_render_pass);
        let distant_field_pass =
            DistantFieldPass::new(device, config, width, height, &quad_render_pass);
        let scene_texture = SceneTexture::new(width, height, device);
        let radiance_old_pass = RadianceRenderOLDPass::new(
            device,
            config,
            width,
            height,
            &quad_render_pass,
            &scene_texture,
        );
        let (texture1, texture2, radians_cascades_old, distant_field_texture) = (
            create_texture(device, width, height),
            create_texture(device, width, height),
            create_texture(device, width, height),
            create_texture(device, width, height),
        );
        Self {
            jfa_pass,
            radiance_old_pass,
            quad_render_pass,
            render_options: Default::default(),
            scene_texture,
            texture1,
            texture2,
            radians_cascades_old,
            seed_pass,
            show_pass,
            distant_field_pass,
            distant_field_texture,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        self.scene_texture.resize(width, height, device);
        self.jfa_pass.resize(width, height, queue);
        self.radiance_old_pass
            .resize(width, height, device, queue, &self.scene_texture);
        (
            self.texture1,
            self.texture2,
            self.radians_cascades_old,
            self.distant_field_texture,
        ) = (
            create_texture(device, width, height),
            create_texture(device, width, height),
            create_texture(device, width, height),
            create_texture(device, width, height),
        )
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        device: &Device,
        queue: &Queue,
    ) {
        let passes = self.render_options.jfa_passes_count;
        self.seed_pass.render(
            encoder,
            device,
            self.scene_texture.view(),
            &self.texture1,
            &self.quad_render_pass,
        );
        for i in 0..passes {
            let (texture1, texture2) = if i % 2 == 0 {
                (&self.texture1, &self.texture2)
            } else {
                (&self.texture2, &self.texture1)
            };
            self.jfa_pass.render(
                encoder,
                device,
                queue,
                texture1,
                texture2,
                2.0f32.powi((passes - i - 1) as i32),
                &self.quad_render_pass,
            );
        }
        self.distant_field_pass.render(
            encoder,
            device,
            &self.texture2,
            &self.distant_field_texture,
            &self.quad_render_pass,
        );

        if self.render_options.radians_old_enabled {
            self.radiance_old_pass.render(
                encoder,
                device,
                queue,
                &self.radians_cascades_old,
                self.render_options.radians_options,
                &self.quad_render_pass,
            );
        }
        let target_texture = match self.render_options.show {
            TextureOption::Texture1 => &self.texture1,
            TextureOption::Texture2 => &self.texture2,
            TextureOption::RadiansCascadesOld => &self.radians_cascades_old,
            TextureOption::SceneTexture => &self.scene_texture.view(),
            TextureOption::DistanceField => &self.distant_field_texture,
        };
        self.show_pass.render(
            encoder,
            device,
            target_texture,
            view,
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
