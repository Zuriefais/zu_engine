use glam::Vec2;
use log::info;
use wgpu::{
    Buffer, BufferUsages, Device, Queue, Texture, TextureView,
    util::{BufferInitDescriptor, DeviceExt},
};

fn create_texture_data(width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let flat_rgba: Vec<u8> = vec![[0u8, 0u8, 0u8, 0u8]; pixel_count]
        .into_iter()
        .flatten()
        .collect();

    // compute padded bytes per row & build a padded buffer
    let unpadded_bytes_per_row = width * 4;
    let bytes_per_row = padded_bytes_per_row(unpadded_bytes_per_row) as usize;
    let mut padded: Vec<u8> = vec![0; bytes_per_row * height as usize];

    // copy each source row into the padded row
    for row in 0..height as usize {
        let src_start = row * (width as usize) * 4;
        let src_end = src_start + (width as usize) * 4;
        let dst_start = row * bytes_per_row;
        padded[dst_start..dst_start + (width as usize) * 4]
            .copy_from_slice(&flat_rgba[src_start..src_end]);
    }
    flat_rgba
}

fn create_texture(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
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
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("diffuse_texture"),
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, texture_view)
}

pub struct SceneTexture {
    texture: Texture,
    texture_view: TextureView,
    texture_data: Vec<u8>,
    size_buffer: Buffer,
}

impl SceneTexture {
    pub fn new(width: u32, height: u32, device: &Device) -> Self {
        let (texture, texture_view) = create_texture(device, width, height);
        let texture_data = create_texture_data(width, height);
        let size_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some(&"Size Buffer"),
            contents: bytemuck::bytes_of(&Vec2::new(width as f32, height as f32)),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self {
            texture,
            texture_view,
            texture_data,
            size_buffer,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, device: &Device) {
        let (texture, texture_view) = create_texture(device, width, height);
        self.texture = texture;
        self.texture_view = texture_view;
        self.texture_data = create_texture_data(width, height);
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
        let center_y = (height as f32 - pos.y) as i32;
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
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: min_x,
                    y: min_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &patch_data,
            wgpu::ImageDataLayout {
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

    pub fn view(&self) -> &TextureView {
        &self.texture_view
    }
}

fn padded_bytes_per_row(unpadded_row_bytes: u32) -> u32 {
    // WebGPU requires bytes_per_row be a multiple of 256
    const ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    ((unpadded_row_bytes + ALIGN - 1) / ALIGN) * ALIGN
}
