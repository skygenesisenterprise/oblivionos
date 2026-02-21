use anyhow::Result;
use calloop::EventLoop;
use log::info;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::state::OblivionState;

pub struct Backend {
    width: u32,
    height: u32,
    surfaces: Arc<Mutex<Vec<Surface>>>,
}

struct Surface {
    id: u32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Backend {
    pub fn init(event_loop: &mut EventLoop<OblivionState>) -> Result<()> {
        info!("Initializing graphics backend");

        let backend = Backend {
            width: 1920,
            height: 1080,
            surfaces: Arc::new(Mutex::new(Vec::new())),
        };

        event_loop.handle().insert_source(
            calloop::timer::Timer::from_duration(std::time::Duration::from_millis(16)),
            move |_, _, state| {
                state.render();
            },
        )?;

        let state = OblivionState::new(backend);
        event_loop.set_state(state);

        info!("Backend initialized with {}x{}", 1920, 1080);
        Ok(())
    }

    pub fn render(&self) {
        let surfaces = self.surfaces.lock();
        for surface in surfaces.iter() {
            self.render_surface(surface);
        }
    }

    fn render_surface(&self, surface: &Surface) {}

    pub fn add_surface(&self, x: i32, y: i32, width: u32, height: u32) -> u32 {
        let mut surfaces = self.surfaces.lock();
        let id = surfaces.len() as u32;
        surfaces.push(Surface {
            id,
            x,
            y,
            width,
            height,
        });
        id
    }

    pub fn remove_surface(&self, id: u32) {
        let mut surfaces = self.surfaces.lock();
        surfaces.retain(|s| s.id != id);
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
