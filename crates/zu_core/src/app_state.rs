use crate::camera::Camera;
use crate::egui_tools::EguiRenderer;
use crate::fragment_render_pass::FragmentRenderPass;
use crate::gui::EngineGui;

use crate::styles::{default_dark::default_dark_theme, gruvbox_egui::gruvbox_dark_theme};
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{ScreenDescriptor, wgpu};
use glam::Vec2;
use log::info;
use std::sync::{Arc, Mutex};

use winit::event::WindowEvent;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;
use winit::window::Window;

pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub scale_factor: f32,
    pub egui_renderer: EguiRenderer,
    pub engine_gui: EngineGui,
    pub window: Arc<Window>,
    pub fragment_render_pass: FragmentRenderPass,
    pub camera: Camera,
    color: [f32; 4],
    paint: bool,
    mouse_pos: Vec2,
    brush_radius: u32,
}

impl AppState {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        info!("Creating App State...");
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");
        info!("Surface created");
        let width = 1920;
        let height = 1080;

        // let _ = window.request_inner_size(PhysicalSize::new(width, height));
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Unable request adapter");

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8Unorm;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("failed to select proper surface texture format!");
        info!("Supported formats: {:?}", swapchain_capabilities.formats);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, &window);

        // Set default egui font
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Adwaita Sans".to_owned(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "styles/AdwaitaSans-Regular.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Adwaita Sans".to_owned());
        egui_renderer.context().set_fonts(fonts);

        // Set egui style
        egui_renderer.context().set_style(default_dark_theme());

        let scale_factor = 1.0;

        let engine_gui = EngineGui::new(egui_renderer.context());
        let camera =
            Camera::from_screen_size(width as f32, height as f32, 0.1, 1000.0, 1.0, Vec2::ZERO);

        let fragment_render_pass = FragmentRenderPass::new(&device, &surface_config, width, height);

        info!("App State created!!");

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            egui_renderer,
            scale_factor,
            engine_gui,
            window,
            fragment_render_pass,
            camera,
            paint: false,
            color: [1.0, 1.0, 1.0, 1.0],
            mouse_pos: Vec2::ZERO,
            brush_radius: 10,
        })
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera
            .update_from_screen_size(width as f32, height as f32);
        self.fragment_render_pass
            .resize(width, height, &self.device, &self.queue);
    }

    pub fn handle_redraw(&mut self) {
        let window = &self.window;
        let width = self.surface_config.width;
        let height = self.surface_config.height;

        if self.paint {
            self.fragment_render_pass.paint(
                self.mouse_pos,
                self.color,
                self.brush_radius,
                width,
                height,
                &self.queue,
            );
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: window.scale_factor() as f32 * self.scale_factor,
        };

        let surface_texture = self.surface.get_current_texture();
        match surface_texture {
            Err(SurfaceError::Outdated) => {
                info!("wgpu surface outdated");
                return;
            }
            Err(_) => {
                surface_texture.expect("Failed to acquire next swap chain texture");
                return;
            }
            Ok(_) => {}
        };
        let surface_texture = surface_texture.unwrap();
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.fragment_render_pass
            .render(&mut encoder, &self.device, &self.queue, &surface_view);

        self.egui_renderer.begin_frame(window);
        self.engine_gui.render_gui(
            &mut self.color,
            &mut self.paint,
            &mut self.mouse_pos,
            &mut self.brush_radius,
        );
        self.egui_renderer.end_frame_and_draw(
            &self.device,
            &self.queue,
            &mut encoder,
            window,
            &surface_view,
            screen_descriptor,
        );

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        window.request_redraw();
    }

    pub fn event(&mut self, event: &WindowEvent) {
        self.egui_renderer.handle_input(&self.window, event);
    }
}
