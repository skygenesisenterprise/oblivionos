use anyhow::Result;
use log::info;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct WaylandManager {
    socket_name: String,
    running: Arc<Mutex<bool>>,
}

impl WaylandManager {
    pub fn new() -> Self {
        Self {
            socket_name: String::new(),
            running: Arc::new(Mutex::new(true)),
        }
    }

    pub fn init() -> Result<Self> {
        info!("Initializing Wayland server");

        let socket_name =
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
        info!("Wayland socket: {}", socket_name);

        let manager = WaylandManager {
            socket_name,
            running: Arc::new(Mutex::new(true)),
        };

        info!("Wayland server initialized");
        Ok(manager)
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    pub fn is_running(&self) -> bool {
        *self.running.lock()
    }

    pub fn stop(&self) {
        *self.running.lock() = false;
    }
}

impl Default for WaylandManager {
    fn default() -> Self {
        Self::new()
    }
}
