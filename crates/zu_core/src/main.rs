mod app;
mod app_state;
pub mod camera;
mod egui_tools;
mod fragment_render_pass;
mod gui;
mod object_render_pass;
mod styles;
mod widgets;

use log::info;

use winit::event_loop::{ControlFlow, EventLoop};

pub fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        extern crate console_error_panic_hook;
        use std::panic;
        panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Trace);
    }
    info!("Starting App");
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(run());
    }
}

async fn run() {
    let event_loop = EventLoop::with_user_event().build().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );

    event_loop.run_app(&mut app).expect("Failed to run app");
}
