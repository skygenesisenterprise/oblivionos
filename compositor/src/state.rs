use crate::backend::Backend;
use crate::wayland::WaylandManager;
use crate::window::WindowManager;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct OblivionState {
    pub backend: Backend,
    pub wayland: Arc<Mutex<WaylandManager>>,
    pub window_manager: Arc<Mutex<WindowManager>>,
    pub running: Arc<Mutex<bool>>,
}

impl OblivionState {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend,
            wayland: Arc::new(Mutex::new(WaylandManager::new())),
            window_manager: Arc::new(Mutex::new(WindowManager::new())),
            running: Arc::new(Mutex::new(true)),
        }
    }

    pub fn render(&mut self) {
        self.backend.render();
    }

    pub fn stop(&self) {
        *self.running.lock() = false;
    }
}

impl calloop::LoopSignal for OblivionState {
    fn signal(&self, signum: libc::c_int) {
        match signum {
            libc::SIGINT | libc::SIGTERM => {
                info!("Received signal {}, shutting down", signum);
                self.stop();
            }
            _ => {}
        }
    }
}
