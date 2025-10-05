#[cfg(target_os = "android")]
use winit::event_loop::EventLoop;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use log::LevelFilter;
    use winit::event_loop::EventLoopBuilder;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(android_logger::Config::default().with_max_level(LevelFilter::Trace));

    let event_loop: EventLoop<AndroidApp> = EventLoopBuilder::new()
        .with_android_app(app)
        .build()
        .expect("Failed to create event loop");

    app::run(event_loop);
}
