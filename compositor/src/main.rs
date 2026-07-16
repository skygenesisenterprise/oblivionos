use anyhow::{Context, Result};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use tracing::{info, error, warn, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub mod buffer;
pub mod effects;
pub mod input;
pub mod output;
pub mod surface;
pub mod vnc;
pub mod window;
pub mod xdg;

use input::{InputEvent, InputHandler};
use vnc::Framebuffer;
use window::WindowManager;
use vnc::{DEFAULT_HEIGHT, DEFAULT_WIDTH};

pub mod render;
use crate::render::{paint_frame, DamageState, Rect};

pub struct Compositor {
    window_manager: Arc<Mutex<WindowManager>>,
    input_handler: InputHandler,
    framebuffer: Arc<RwLock<Framebuffer>>,
    /// mpsc receiver populated by Stage 3 VNC clients (KeyEvent / PointerEvent).
    /// `Some` when `OBLIVION_VNC=1` was set or `Compositor::with_input_channel`
    /// was used externally.
    input_rx: Option<tokio::sync::mpsc::Receiver<InputEvent>>,
    /// Damage state shared with the VNC server task. Always present
    /// even when OBLIVION_VNC is off, so the renderer doesn't have
    /// to special-case it.
    damage: Arc<DamageState>,
}

impl Compositor {
    pub fn new() -> Result<Self> {
        let fb = Arc::new(RwLock::new(Framebuffer::new(
            DEFAULT_WIDTH,
            DEFAULT_HEIGHT,
            [16, 16, 24, 255], // matches render::BG so first paint is a no-op colour-wise
        )));
        let damage = Arc::new(DamageState::new((DEFAULT_WIDTH as u32, DEFAULT_HEIGHT as u32)));
        Ok(Self::from_parts(WindowManager::new(), InputHandler::new(), fb, damage, None))
    }

    /// Build a compositor from a framebuffer + optional input channel.
    /// Used by `main` to bind the in-process VNC server's mpsc Sender
    /// without paying for the default-framebuffer allocation in `new()`.
    pub fn with_input_channel(
        fb: Arc<RwLock<Framebuffer>>,
        input_rx: tokio::sync::mpsc::Receiver<InputEvent>,
        damage: Arc<DamageState>,
    ) -> Result<Self> {
        Ok(Self::from_parts(
            WindowManager::new(),
            InputHandler::new(),
            fb,
            damage,
            Some(input_rx),
        ))
    }

    fn from_parts(
        wm: WindowManager,
        input_handler: InputHandler,
        framebuffer: Arc<RwLock<Framebuffer>>,
        damage: Arc<DamageState>,
        input_rx: Option<tokio::sync::mpsc::Receiver<InputEvent>>,
    ) -> Self {
        Self {
            window_manager: Arc::new(Mutex::new(wm)),
            input_handler,
            framebuffer,
            damage,
            input_rx,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting OblivionOS Compositor event loop");

        // Render pipeline: every ~33 ms we walk WindowManager and paint
        // each visible window into the shared Framebuffer, then publish
        // the resulting damage list. The in-process VNC server picks it
        // up via Arc<DamageState> and ships it as RFB FramebufferUpdate
        // messages (full or incremental, per client request).
        let mut last_swap = std::time::Instant::now();
        loop {
            // 1. Drain input events from the (optional) VNC channel.
            if let Some(rx) = self.input_rx.as_mut() {
                let drained: Vec<InputEvent> = {
                    let mut out = Vec::new();
                    loop {
                        match rx.try_recv() {
                            Ok(ev) => out.push(ev),
                            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                                warn!("VNC input channel disconnected; dropping receiver");
                                return Ok(());
                            }
                        }
                    }
                    out
                };
                for ev in drained {
                    self.input_handler.queue_event(ev);
                }
                self.input_handler.process();
            }

            // 2. Paint at ≈30 fps.
            if last_swap.elapsed() >= std::time::Duration::from_millis(33) {
                // Hold BOTH the framebuffer write lock AND the
                // window_manager mutex while painting. They form a
                // single "rendering" critical section: no VNC client
                // sees a half-updated window mid-paint, and no other
                // thread mutates WindowManager mid-iteration.
                //
                // Snapshot the framebuffer width/height once, INSIDE
                // the write-lock block, so we don't reach for a second
                // fb.read() afterward (which would re-enter the lock
                // and contend with any VNC task that read-locked for
                // a snapshot in between).
                let (damage, full_rect) = {
                    let mut fb = self.framebuffer.write();
                    let wm = self.window_manager.lock();
                    let d = paint_frame(&mut fb, &wm);
                    let full = Rect::full(fb.width as u32, fb.height as u32);
                    (d, full)
                };
                self.damage.publish(damage, full_rect);
                last_swap = std::time::Instant::now();
            }

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

    // Stage 3 wiring: when OBLIVION_VNC=1 / true, spawn an in-process
    // VNC server on a dedicated std::thread that owns its own small
    // current-thread tokio runtime. The receiver end of the input
    // channel lives on this thread and is drained non-blockingly from
    // Compositor::run. See compositor::vnc for the protocol.
    let enable_vnc = std::env::var("OBLIVION_VNC")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let vnc_bind = std::env::var("OBLIVION_VNC_BIND")
        .unwrap_or_else(|_| "127.0.0.1:5900".to_string());
    let vnc_name = std::env::var("OBLIVION_VNC_NAME")
        .unwrap_or_else(|_| "OblivionOS".to_string());

    let mut compositor = if enable_vnc {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let fb = Arc::new(RwLock::new(Framebuffer::new(
            DEFAULT_WIDTH,
            DEFAULT_HEIGHT,
            [16, 16, 24, 255],
        )));
        let damage = Arc::new(DamageState::new((DEFAULT_WIDTH as u32, DEFAULT_HEIGHT as u32)));
        let cfg = vnc::VncConfig {
            bind_addr: vnc_bind.clone(),
            name: vnc_name,
            fb: fb.clone(),
            input_tx: tx,
            damage: damage.clone(),
        };
        info!(addr = %vnc_bind, "OBLIVION_VNC: spawning in-process VNC server");
        std::thread::Builder::new()
            .name("oblivion-vnc".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("build vnc tokio runtime: {e}");
                        return;
                    }
                };
                rt.block_on(async move {
                    if let Err(e) = vnc::run_server(cfg).await {
                        error!("vnc server: {e:#}");
                    }
                });
                // rt drops at end of scope, joining its worker thread.
            })
            .context("spawn vnc thread")?;
        Compositor::with_input_channel(fb, rx, damage)?
    } else {
        Compositor::new()?
    };

    info!("OblivionOS Compositor running");

    if let Err(e) = compositor.run() {
        error!("Compositor error: {}", e);
        return Err(e);
    }

    Ok(())
}