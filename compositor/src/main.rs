use anyhow::Result;
use calloop::EventLoop;
use log::info;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

mod wayland;
mod window;

struct AppState {
    wayland: Arc<Mutex<wayland::WaylandManager>>,
    window_manager: Arc<Mutex<window::WindowManager>>,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!(
        "Starting OblivionOS Compositor v{}",
        env!("CARGO_PKG_VERSION")
    );

    let wayland_manager = wayland::WaylandManager::init()?;

    let mut state = AppState {
        wayland: Arc::new(Mutex::new(wayland_manager)),
        window_manager: Arc::new(Mutex::new(window::WindowManager::new())),
    };

    let mut event_loop: EventLoop<AppState> = EventLoop::try_new()?;

    info!("OblivionOS Compositor initialized successfully");

    event_loop.run(Duration::from_millis(16), &mut state, |state| {
        // Render loop - will be expanded with actual rendering
        let _ = state.wayland.lock();
        let _ = state.window_manager.lock();
    })?;

    Ok(())
}
