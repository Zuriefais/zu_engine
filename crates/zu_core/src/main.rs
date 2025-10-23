use log::info;
use winit::event_loop::{ControlFlow, EventLoop};
use zu_core::{app::App, start_puffin_server};

#[cfg(target_os = "windows")]
use win_dialog::WinDialog;

#[cfg(target_os = "windows")]
use backtrace::Backtrace;

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
    #[cfg(target_os = "windows")]
    {
        // Set the custom panic hook only on Windows
        panic::set_hook(Box::new(|panic_info: &PanicInfo| {
            // Capture panic details
            let message = format!(
                "Panic occurred!\n\nMessage: {}\nLocation: {}\n\nBacktrace:\n{:?}",
                panic_info
                    .payload()
                    .downcast_ref::<&str>()
                    .unwrap_or(&"Unknown"),
                panic_info
                    .location()
                    .unwrap_or(&std::panic::Location::caller()),
                Backtrace::new()
            );

            // Display Win32 MessageBox dialog via win_dialog
            let _ = WinDialog::new(&message)
                .with_header("Application Panic")
                .show();
        }));
    }
    start_puffin_server();
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

    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );

    event_loop.run_app(&mut app).expect("Failed to run app");
}
