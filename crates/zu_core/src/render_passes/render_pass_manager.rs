use egui_probe::EguiProbe;
use glam::Vec2;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource,
    CommandEncoder, Device, Queue, Sampler, Texture, TextureView,
};

use crate::{
    render_passes::{
        distant_field_pass::{self, DistantFieldPass},
        jfa_pass::{self, JfaRenderPass},
        quad_vertex::QuadVertexRenderPass,
        radiance_render_old_pass::{RadianceRenderOLDPass, RadiansOptions},
        seed_pass::{self, SeedRenderPass},
        show_pass::{self, ShowRenderPass},
    },
    texture_manager::{self, ManagedTexture, TextureManager},
};

fn create_texture(
    device: &Device,
    width: u32,
    height: u32,
    texture_bind_group_layout: &BindGroupLayout,
    sampler: &Sampler,
) -> (TextureView, BindGroup) {
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
    let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Texture Bind Group"),
        layout: texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&sampler),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&texture_view),
            },
        ],
    });
    (texture_view, texture_bind_group)
}

#[derive(Debug, Clone, EguiProbe)]
pub struct RenderOptions {
    radians_options: RadiansOptions,
    radians_old_enabled: bool,
    jfa_passes_count: u32,
    show: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            radians_options: Default::default(),
            jfa_passes_count: 9,
            radians_old_enabled: true,
            show: "RadiansCascadesOld".into(),
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
    texture_manager: TextureManager,
}

impl RenderPassManager {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> RenderPassManager {
        let mut texture_manager = TextureManager::new(device);
        let quad_render_pass = QuadVertexRenderPass::new(device);
        let jfa_pass = JfaRenderPass::new(
            device,
            width,
            height,
            &quad_render_pass,
            &mut texture_manager,
        );
        let seed_pass = SeedRenderPass::new(device, &texture_manager, &quad_render_pass);
        let show_pass = ShowRenderPass::new(device, config, &quad_render_pass);
        let distant_field_pass = DistantFieldPass::new(
            device,
            &quad_render_pass,
            width,
            height,
            &mut texture_manager,
        );

        let radiance_old_pass =
            RadianceRenderOLDPass::new(device, width, height, &quad_render_pass, &texture_manager);

        Self {
            jfa_pass,
            radiance_old_pass,
            quad_render_pass,
            render_options: Default::default(),
            seed_pass,
            show_pass,
            distant_field_pass,
            texture_manager,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue) {
        self.texture_manager.resize(device, (width, height));
        self.jfa_pass.resize(width, height);
        self.radiance_old_pass.resize(width, height);
    }

    pub fn render(&mut self, view: &TextureView, encoder: &mut CommandEncoder, device: &Device) {
        self.seed_pass
            .render(encoder, &self.texture_manager, &self.quad_render_pass);
        self.jfa_pass
            .multi_render(encoder, &self.quad_render_pass, &self.texture_manager);
        self.distant_field_pass.render(
            encoder,
            device,
            &self.texture_manager,
            &self.quad_render_pass,
        );

        if self.render_options.radians_old_enabled {
            self.radiance_old_pass.render(
                encoder,
                self.render_options.radians_options,
                &self.texture_manager,
                &self.quad_render_pass,
            );
        }
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
