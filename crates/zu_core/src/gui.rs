use crate::widgets::usage_diagnostics::UsageDiagnostics;
use egui::Context;
use egui::Widget;

pub struct EngineGui {
    egui_context: Context,
}

impl EngineGui {
    pub fn new(context: &Context) -> Self {
        Self {
            egui_context: context.clone(),
        }
    }

    pub fn render_gui(&mut self) {
        egui::Window::new("Engine Window")
            .show(&self.egui_context, |ui| UsageDiagnostics {}.ui(ui));
    }
}
