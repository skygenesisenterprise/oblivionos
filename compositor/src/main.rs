use anyhow::Result;
use log::info;
use std::panic;
use std::sync::Arc;
use parking_lot::Mutex;

mod backend;
mod state;
mod window;
mod wayland;

use state::OblivionState;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    panic::set_hook(Box::new(|info| {
        log::error!("Panic: {}", info);
    }));

    info!("Starting OblivionOS Compositor v{}", env!("CARGO_PKG_VERSION"));

    let mut event_loop = calloop::EventLoop::<OblivionState>::new()?;

    backend::Backend::init(&mut event_loop)?;
    wayland::WaylandManager::init(&mut event_loop)?;

    info!("OblivionOS Compositor initialized successfully");

    event_loop.run(None, &mut |_, state| {
        state.render();
    })?;

    Ok(())
}
