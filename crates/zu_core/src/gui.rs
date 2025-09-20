use crate::widgets::usage_diagnostics::UsageDiagnostics;
use egui::Context;
use egui::Widget;
use glam::Vec2;
use glam::Vec4;

pub struct EngineGui {
    egui_context: Context,
}

impl EngineGui {
    pub fn new(context: &Context) -> Self {
        Self {
            egui_context: context.clone(),
        }
    }

    pub fn render_gui(
        &mut self,
        color: &mut [f32; 4],
        paint: &mut bool,
        pointer_pos: &mut Vec2,
        brush_radius: &mut u32,
    ) {
        *pointer_pos = {
            if let Some(pos) = self.egui_context.pointer_latest_pos() {
                let (x, y) = (pos.x, pos.y);
                Vec2 { x, y } * self.egui_context.pixels_per_point()
            } else {
                Vec2::ZERO
            }
        };
        self.egui_context.input(|input| {
            *paint = input.pointer.primary_down();
        });
        egui::Window::new("Engine Window").show(&self.egui_context, |ui| {
            ui.color_edit_button_rgba_unmultiplied(color);
            ui.add(egui::Slider::new(brush_radius, 0..=120).text("brush radius"));
            UsageDiagnostics {}.ui(ui);
        });
    }
}
