use anyhow::Result;
use calloop::EventLoop;
use log::info;
use parking_lot::Mutex;
use std::sync::Arc;
use wayland_server::Display;

use crate::state::OblivionState;

pub struct WaylandManager {
    display: Option<Display>,
    socket_name: String,
}

impl WaylandManager {
    pub fn new() -> Self {
        Self {
            display: None,
            socket_name: String::new(),
        }
    }

    pub fn init(event_loop: &mut EventLoop<OblivionState>) -> Result<()> {
        info!("Initializing Wayland server");

        let mut display = Display::new();

        let socket_name = display.add_socket_auto()?;
        info!("Wayland socket created: {}", socket_name);

        let manager = WaylandManager {
            display: Some(display),
            socket_name,
        };

        let state = event_loop.state();
        state.wayland = Arc::new(Mutex::new(manager));

        info!("Wayland server initialized");
        Ok(())
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    pub fn get_display(&self) -> Option<&Display> {
        self.display.as_ref()
    }
}

impl Default for WaylandManager {
    fn default() -> Self {
        Self::new()
    }
}
