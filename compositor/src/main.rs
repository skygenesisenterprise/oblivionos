use anyhow::{Context, Result};
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub mod buffer;
pub mod effects;
pub mod input;
pub mod output;
pub mod surface;
pub mod window;
pub mod xdg;

use window::WindowManager;

pub struct Compositor {
    window_manager: Arc<Mutex<WindowManager>>,
}

impl Compositor {
    pub fn new() -> Result<Self> {
        let window_manager = WindowManager::new();

        info!("OblivionOS Compositor initialized successfully");

        Ok(Self {
            window_manager: Arc::new(Mutex::new(window_manager)),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting OblivionOS Compositor event loop");
        
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    pub fn create_window(&mut self, title: String, app_id: String) -> window::WindowId {
        let mut wm = self.window_manager.lock();
        wm.create_window(title, app_id)
    }

    pub fn get_window(&self, id: window::WindowId) -> Option<window::WindowId> {
        let wm = self.window_manager.lock();
        wm.get_window(id).map(|_| id)
    }
}

pub fn setup_logging() -> Result<()> {
    let log_dir = directories::ProjectDirs::from("com", "oblivionos", "compositor")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "oblivion-compositor.log",
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
        )
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()));

    subscriber.try_init().ok();

    std::mem::forget(_guard);

    Ok(())
}

fn main() -> Result<()> {
    setup_logging()?;

    info!("Starting OblivionOS Compositor v{}", env!("CARGO_PKG_VERSION"));
    
    info!("Initializing compositor...");
    
    let mut compositor = Compositor::new()?;
    
    info!("OblivionOS Compositor running");
    
    if let Err(e) = compositor.run() {
        error!("Compositor error: {}", e);
        return Err(e);
    }

    Ok(())
}