use glam::Vec2;
use indexmap::IndexMap;
use log::info;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Device, Queue, Sampler,
    Texture, TextureView,
};

pub enum TextureType {
    Standart,
    SceneTexture,
}

pub enum ManagedTexture {
    Standart(EngineTexture),
    SceneTexture(SceneTexture),
}

impl ManagedTexture {
    pub fn view(&self) -> &TextureView {
        match self {
            ManagedTexture::Standart(engine_texture) => &engine_texture.view,
            ManagedTexture::SceneTexture(scene_texture) => &scene_texture.texture.view,
        }
    }
    pub fn bind_group(&self) -> &BindGroup {
        match self {
            ManagedTexture::Standart(engine_texture) => &engine_texture.bind_group,
            ManagedTexture::SceneTexture(scene_texture) => &scene_texture.texture.bind_group,
        }
    }
}

impl ManagedTexture {
    pub fn standard(&self) -> Option<&EngineTexture> {
        if let ManagedTexture::Standart(standart) = self {
            Some(&standart)
        } else {
            None
        }
    }

    pub fn scene(&self) -> Option<&SceneTexture> {
        if let ManagedTexture::SceneTexture(scene) = self {
            Some(&scene)
        } else {
            None
        }
    }
}

pub struct EngineTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub bind_group: BindGroup,
}

impl EngineTexture {
    pub fn new(
        name: &str,
        resolution: (u32, u32),
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: resolution.0,
            height: resolution.1,
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
            label: Some(name),
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        Self {
            view,
            bind_group,
            texture,
        }
    }
}

pub struct SceneTexture {
    texture: EngineTexture,
    texture_data: Vec<u8>,
}

impl SceneTexture {
    pub fn new(
        name: &str,
        resolution: (u32, u32),
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        sampler: &Sampler,
    ) -> Self {
        let texture = EngineTexture::new(name, resolution, device, bind_group_layout, sampler);
        let pixel_count = (resolution.0 * resolution.1) as usize;
        let flat_rgba: Vec<u8> = vec![[0u8, 0u8, 0u8, 0u8]; pixel_count]
            .into_iter()
            .flatten()
            .collect();

        // compute padded bytes per row & build a padded buffer
        let unpadded_bytes_per_row = resolution.0 * 4;
        let bytes_per_row = padded_bytes_per_row(unpadded_bytes_per_row) as usize;
        let mut padded: Vec<u8> = vec![0; bytes_per_row * resolution.1 as usize];

        // copy each source row into the padded row
        for row in 0..resolution.1 as usize {
            let src_start = row * (resolution.0 as usize) * 4;
            let src_end = src_start + (resolution.0 as usize) * 4;
            let dst_start = row * bytes_per_row;
            padded[dst_start..dst_start + (resolution.0 as usize) * 4]
                .copy_from_slice(&flat_rgba[src_start..src_end]);
        }

        Self {
            texture,
            texture_data: flat_rgba,
        }
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
        // Convert color from f32 [0.0, 1.0] to u8 [0, 255]
        let rgba = [
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            255,
        ];

        let center_x = pos.x as i32;
        let center_y = pos.y as i32;
        let radius_sq = (brush_radius as i32).pow(2);

        info!("Painting at {}, {}, color: {:?}", center_x, center_y, rgba);

        let min_x = (center_x - brush_radius as i32).max(0) as u32;
        let max_x = (center_x + brush_radius as i32).min((width - 1) as i32) as u32;
        let min_y = (center_y - brush_radius as i32).max(0) as u32;
        let max_y = (center_y + brush_radius as i32).min((height - 1) as i32) as u32;

        if min_x > max_x || min_y > max_y {
            return;
        }

        let rect_width = max_x - min_x + 1;
        let rect_height = max_y - min_y + 1;

        let bytes_per_row = padded_bytes_per_row(rect_width * 4);
        let mut patch_data = vec![0u8; (bytes_per_row * rect_height) as usize];

        for y_in_rect in 0..rect_height {
            for x_in_rect in 0..rect_width {
                let tex_x = min_x + x_in_rect;
                let tex_y = min_y + y_in_rect;

                let dx = tex_x as i32 - center_x;
                let dy = tex_y as i32 - center_y;

                let patch_idx_start = (y_in_rect * bytes_per_row + x_in_rect * 4) as usize;

                if dx * dx + dy * dy <= radius_sq {
                    patch_data[patch_idx_start..patch_idx_start + 4].copy_from_slice(&rgba);
                    let cpu_idx = ((tex_y * width) + tex_x) as usize * 4;
                    self.texture_data[cpu_idx..cpu_idx + 4].copy_from_slice(&rgba);
                } else {
                    // If outside, use the existing color from our CPU-side copy
                    let cpu_idx = ((tex_y * width) + tex_x) as usize * 4;
                    let old_color = &self.texture_data[cpu_idx..cpu_idx + 4];
                    patch_data[patch_idx_start..patch_idx_start + 4].copy_from_slice(old_color);
                }
            }
        }

        // 4. Write the entire patch to the GPU texture in a single call
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: min_x,
                    y: min_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &patch_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rect_height),
            },
            wgpu::Extent3d {
                width: rect_width,
                height: rect_height,
                depth_or_array_layers: 1,
            },
        );
    }
}

pub struct TextureManager {
    textures: IndexMap<String, ManagedTexture>,
    texture_bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    num: usize,
}

impl TextureManager {
    pub fn new(device: &Device) -> Self {
        Self {
            textures: Default::default(),
            texture_bind_group_layout: device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("Texture Bind Group Layout"),
                    entries: &[
                        // sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                },
            ),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            num: 0,
        }
    }

    pub fn create_texture(
        &mut self,
        name: &str,
        resolution: (u32, u32),
        device: &Device,
        texture_type: TextureType,
    ) -> usize {
        match texture_type {
            TextureType::Standart => {
                self.textures.insert(
                    name.to_string(),
                    ManagedTexture::Standart(EngineTexture::new(
                        name,
                        resolution,
                        device,
                        &self.texture_bind_group_layout,
                        &self.sampler,
                    )),
                );
            }
            TextureType::SceneTexture => {
                self.textures.insert(
                    name.to_string(),
                    ManagedTexture::SceneTexture(SceneTexture::new(
                        name,
                        resolution,
                        device,
                        &self.texture_bind_group_layout,
                        &self.sampler,
                    )),
                );
            }
        }

        self.num += 1;
        self.num - 1
    }

    pub fn get_texture(&self, name: &str) -> Option<&ManagedTexture> {
        self.textures.get(name)
    }

    pub fn get_texture_by_index(&self, i: usize) -> Option<&ManagedTexture> {
        self.textures.get_index(i).map(|(_, v)| v)
    }

    pub fn get_texture_mut(&mut self, name: &str) -> Option<&mut ManagedTexture> {
        self.textures.get_mut(name)
    }

    pub fn get_texture_by_index_mut(&mut self, i: usize) -> Option<&mut ManagedTexture> {
        self.textures.get_index_mut(i).map(|(_, v)| v)
    }

    pub fn resize(&mut self, device: &Device, resolution: (u32, u32)) {
        for (name, texture) in self.textures.iter_mut() {
            match texture {
                ManagedTexture::Standart(engine_texture) => {
                    *engine_texture = EngineTexture::new(
                        name,
                        resolution,
                        device,
                        &self.texture_bind_group_layout,
                        &self.sampler,
                    )
                }
                ManagedTexture::SceneTexture(scene_texture) => {
                    *scene_texture = SceneTexture::new(
                        name,
                        resolution,
                        device,
                        &self.texture_bind_group_layout,
                        &self.sampler,
                    )
                }
            }
        }
    }

    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.texture_bind_group_layout
    }
}

fn padded_bytes_per_row(unpadded_row_bytes: u32) -> u32 {
    // WebGPU requires bytes_per_row be a multiple of 256
    const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded_row_bytes.div_ceil(ALIGN) * ALIGN
}
