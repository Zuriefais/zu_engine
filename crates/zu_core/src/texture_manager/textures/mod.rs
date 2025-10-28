use crate::texture_manager::{
    BindGroupLayouts,
    textures::{
        scene_texture::SceneTexture, standard::StandardTexture, standard_f16::StandardTextureF16,
        standart_rg16::StandardTextureRGF16,
    },
};

use wgpu::{BindGroup, Device, Sampler, TextureView};

pub mod scene_texture;
pub mod standard;
pub mod standard_f16;
pub mod standart_rg16;

pub enum TextureType {
    Standard,
    StandardF16,
    StandardRGF16,
    SceneTexture,
}

pub trait EngineTexture {
    fn view(&self) -> &TextureView;
    fn bind_group(&self) -> &BindGroup;
    fn compute_bind_group(&self) -> &BindGroup;
    fn compute_storage_group_f16(&self) -> Option<&BindGroup>;
    fn compute_storage_group_rgf16(&self) -> Option<&BindGroup>;
    fn compute_storage_mut_group_f32(&self) -> Option<&BindGroup>;
    fn compute_storage_mut_group_f16(&self) -> Option<&BindGroup>;
    fn compute_storage_mut_group_rgf16(&self) -> Option<&BindGroup>;
    fn resize(
        &mut self,
        resolution: (u32, u32),
        device: &Device,
        bind_group_layouts: &BindGroupLayouts,
        sampler: &Sampler,
        resolution_scale: f32,
        name: &str,
    );
    fn resolution_scale(&self) -> f32;
    fn resolution(&self) -> (u32, u32);
}

pub enum ManagedTexture {
    Standard(StandardTexture),
    StandardF16(StandardTextureF16),
    StandardRGF16(StandardTextureRGF16),
    SceneTexture(SceneTexture),
}

impl EngineTexture for ManagedTexture {
    fn view(&self) -> &TextureView {
        self.as_engine_texture().view()
    }
    fn bind_group(&self) -> &BindGroup {
        self.as_engine_texture().bind_group()
    }

    fn compute_bind_group(&self) -> &BindGroup {
        self.as_engine_texture().compute_bind_group()
    }

    fn compute_storage_group_f16(&self) -> Option<&BindGroup> {
        self.as_engine_texture().compute_storage_group_f16()
    }

    fn compute_storage_mut_group_f32(&self) -> Option<&BindGroup> {
        self.as_engine_texture().compute_storage_mut_group_f32()
    }

    fn compute_storage_mut_group_f16(&self) -> Option<&BindGroup> {
        self.as_engine_texture().compute_storage_mut_group_f16()
    }

    fn resize(
        &mut self,
        resolution: (u32, u32),
        device: &Device,
        bind_group_layouts: &BindGroupLayouts,
        sampler: &Sampler,
        resolution_scale: f32,
        name: &str,
    ) {
        self.as_engine_texture_mut().resize(
            resolution,
            device,
            bind_group_layouts,
            sampler,
            resolution_scale,
            name,
        );
    }

    fn resolution_scale(&self) -> f32 {
        self.as_engine_texture().resolution_scale()
    }

    fn resolution(&self) -> (u32, u32) {
        self.as_engine_texture().resolution()
    }

    fn compute_storage_group_rgf16(&self) -> Option<&BindGroup> {
        self.as_engine_texture().compute_storage_group_rgf16()
    }

    fn compute_storage_mut_group_rgf16(&self) -> Option<&BindGroup> {
        self.as_engine_texture().compute_storage_mut_group_rgf16()
    }
}

impl ManagedTexture {
    pub fn new(
        name: &str,
        resolution: (u32, u32),
        device: &Device,
        bind_group_layouts: &BindGroupLayouts,
        sampler: &Sampler,
        resolution_scale: f32,
        texture_type: TextureType,
    ) -> Self {
        match texture_type {
            TextureType::Standard => ManagedTexture::Standard(StandardTexture::new(
                name,
                resolution,
                device,
                bind_group_layouts,
                sampler,
                resolution_scale,
            )),
            TextureType::StandardF16 => ManagedTexture::StandardF16(StandardTextureF16::new(
                name,
                resolution,
                device,
                bind_group_layouts,
                sampler,
                resolution_scale,
            )),
            TextureType::SceneTexture => ManagedTexture::SceneTexture(SceneTexture::new(
                name,
                resolution,
                device,
                bind_group_layouts,
                sampler,
                resolution_scale,
            )),
            TextureType::StandardRGF16 => ManagedTexture::StandardRGF16(StandardTextureRGF16::new(
                name,
                resolution,
                device,
                bind_group_layouts,
                sampler,
                resolution_scale,
            )),
        }
    }

    pub fn as_engine_texture(&self) -> &dyn EngineTexture {
        self
    }

    pub fn as_engine_texture_mut(&mut self) -> &mut dyn EngineTexture {
        self
    }

    pub fn standard(&self) -> Option<&StandardTexture> {
        if let ManagedTexture::Standard(standart) = self {
            Some(standart)
        } else {
            None
        }
    }

    pub fn standard_f16(&self) -> Option<&StandardTextureF16> {
        if let ManagedTexture::StandardF16(standart) = self {
            Some(standart)
        } else {
            None
        }
    }

    pub fn standard_rgf16(&self) -> Option<&StandardTextureRGF16> {
        if let ManagedTexture::StandardRGF16(standart) = self {
            Some(standart)
        } else {
            None
        }
    }

    pub fn scene(&self) -> Option<&SceneTexture> {
        if let ManagedTexture::SceneTexture(scene) = self {
            Some(scene)
        } else {
            None
        }
    }
}
