//! compositor/src/render.rs
//!
//! Stage 3.1 of the VNC roadmap: a software painter that walks the
//! `WindowManager`'s visible windows and writes BGRA pixels into a
//! `Framebuffer`, returning the list of `Rect`s that changed. The list
//! is the input to a shared damage state which the in-process VNC
//! server uses to honor incremental FramebufferUpdateRequests.
//!
//! This is intentionally NOT smithay / calloop / wayland-server yet
//! (those will land in a separate milestone once Wayland FDs are
//! wired up). The contract with the rest of the compositor is just:
//!     paint_frame(&mut fb, &wm) -> Vec<Rect>
//! — produce BGRA pixels and a damage list. Everything else is real
//! renderer plumbing that will slot in behind this signature.

use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::vnc::Framebuffer;
use crate::window::{WindowManager, WindowState};

/// A rectangular damage region in pixel coordinates. The framebuffer
/// origin is top-left; x/y may be negative to mark regions *outside*
/// the framebuffer (the caller is responsible for clamping them when
/// they hit the wire).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn full(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, w: width, h: height }
    }
}

/// Damage state shared between the compositor's render loop and the
/// VNC server thread. Both threads see the same `Arc<DamageState>`.
///
/// Lock ordering: only `damage.rects` is contended. Compositor
/// holds the parking_lot lock for ≤ a few microseconds (extend +
/// cap), then `notify.notify_waiters()`. VNC tasks hold it for the
/// duration of a drain, which is also short. No cross-thread lock
/// ordering risk.
pub struct DamageState {
    /// Accumulated dirty rects since the last successful FramebufferUpdate.
    pub rects: Mutex<Vec<Rect>>,
    /// Wakes any VNC task parked on `notified().await`.
    pub notify: Notify,
    /// Framebuffer dimensions, refreshed on every publish. Used to
    /// collapse the damage list to a single full-screen rect when it
    /// would exceed `MAX_DAMAGE_RECTS`.
    pub fb_size: Mutex<(u32, u32)>,
}

/// Soft cap on the damage Vec. If exceeded we collapse to a single
/// full-screen rect so we don't slowly leak memory on a long-running
/// compositor with no VNC client connected.
pub const MAX_DAMAGE_RECTS: usize = 50;

impl DamageState {
    pub fn new(initial_size: (u32, u32)) -> Self {
        Self {
            rects: Mutex::new(Vec::new()),
            notify: Notify::new(),
            fb_size: Mutex::new(initial_size),
        }
    }

    /// Compositor-side: append a frame's damage rects, cap if
    /// necessary, and notify any waiting VNC tasks. The full-frame
    /// rect passed in here is what we fall back to when the cap
    /// triggers, so callers should pass `Rect::full(fb.width, fb.height)`.
    pub fn publish(&self, new_rects: Vec<Rect>, full_rect: Rect) {
        {
            let mut guard = self.rects.lock();
            guard.extend(new_rects);
            // Refresh the recorded fb size whenever we publish.
            *self.fb_size.lock() = (full_rect.w, full_rect.h);
            if guard.len() > MAX_DAMAGE_RECTS {
                guard.clear();
                guard.push(full_rect);
            }
        }
        // notify_waiters wakes every task currently parked on notified().
        // It's non-blocking and idempotent.
        self.notify.notify_waiters();
    }

    /// VNC-side: atomically take the entire damage list. Caller now
    /// owns the rects; subsequent paints add to a fresh list.
    pub fn drain(&self) -> Vec<Rect> {
        let mut guard = self.rects.lock();
        std::mem::take(&mut *guard)
    }

    /// VNC-side: peek at the current damage without removing it.
    pub fn peek(&self) -> Vec<Rect> {
        self.rects.lock().clone()
    }

    /// VNC-side: framebuffer dimensions, for collapsing damage if the
    /// client asks for "all" without knowing the real size.
    pub fn fb_size(&self) -> (u32, u32) {
        *self.fb_size.lock()
    }
}

// ----------------------------------------------------------------------------
// Painter
// ----------------------------------------------------------------------------

/// Background colour — deep navy.
const BG: [u8; 4] = [16, 16, 24, 255];
/// 1-px window border colour.
const BORDER: [u8; 4] = [40, 40, 56, 255];
/// Window-body palette (deterministic per-window-id).
const WINDOW_PALETTE: [[u8; 4]; 6] = [
    [56, 78, 120, 255],
    [72, 56, 96, 255],
    [56, 100, 96, 255],
    [108, 88, 56, 255],
    [72, 96, 56, 255],
    [56, 56, 84, 255],
];
/// Title-bar height in pixels (no text rendering — just a darker
/// stripe at the top of each window).
const TITLE_BAR_H: u32 = 24;

/// Paint one compositor frame into `fb` and return the rects that
/// changed (always includes the full-screen background rect plus each
/// window's bounding rect, which is fine for v0 — see Stage 3.1
/// "real renderer" milestone for incremental damage tracking).
pub fn paint_frame(fb: &mut Framebuffer, wm: &WindowManager) -> Vec<Rect> {
    let mut damage = Vec::with_capacity(wm.window_count() + 1);

    // Background.
    fb.fill_solid(BG);
    damage.push(Rect::full(fb.width as u32, fb.height as u32));

    // Iterate in stacking order so later windows draw on top.
    let stacked = wm.get_stacked_windows();
    for window in stacked.iter().copied() {
        if !window.visible || matches!(window.state, WindowState::Minimized) {
            continue;
        }
        let win_w = window.width;
        let win_h = window.height;
        if win_w == 0 || win_h == 0 {
            continue;
        }

        // Pick a deterministic palette entry from the window id.
        let color = WINDOW_PALETTE[(window.id.as_u32() as usize) % WINDOW_PALETTE.len()];
        let title_color = [
            color[0].saturating_sub(20),
            color[1].saturating_sub(20),
            color[2].saturating_sub(20),
            255,
        ];

        // Compute draw origin, clamped to the framebuffer so we never
        // scribble outside it. Negative x/y are allowed (e.g. a
        // window half off-screen); we just don't draw those pixels.
        let fb_w = fb.width as i32;
        let fb_h = fb.height as i32;
        let ox = window.x;
        let oy = window.y;
        let x0 = ox.max(0);
        let y0 = oy.max(0);
        let x1 = (ox + win_w as i32).min(fb_w);
        let y1 = (oy + win_h as i32).min(fb_h);
        if x1 <= x0 || y1 <= y0 {
            continue; // window entirely off-screen
        }
        let draw_w = (x1 - x0) as u32;
        let draw_h = (y1 - y0) as u32;

        // Body.
        fill_rect(fb, x0 as u32, y0 as u32, draw_w, draw_h, color);
        // 1-px border.
        if draw_w >= 1 {
            fill_rect(fb, x0 as u32, y0 as u32, draw_w, 1, BORDER);
            fill_rect(fb, x0 as u32, y1 as u32 - 1, draw_w, 1, BORDER);
        }
        if draw_h >= 1 {
            fill_rect(fb, x0 as u32, y0 as u32, 1, draw_h, BORDER);
            fill_rect(fb, x1 as u32 - 1, y0 as u32, 1, draw_h, BORDER);
        }
        // Title bar (clipped to fb).
        let tb_h = TITLE_BAR_H.min(draw_h);
        fill_rect(fb, x0 as u32, y0 as u32, draw_w, tb_h, title_color);

        damage.push(Rect::new(ox, oy, win_w, win_h));
    }

    damage
}

/// Fill `color` into the rect `(x, y, w, h)` of `fb`. Bounds-checked;
/// silently skips rects that fall outside the framebuffer.
fn fill_rect(fb: &mut Framebuffer, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]) {
    let fw = fb.width as u32;
    let fh = fb.height as u32;
    if x >= fw || y >= fh || w == 0 || h == 0 {
        return;
    }
    let max_w = (fw - x).min(w);
    let max_h = (fh - y).min(h);
    let stride = fb.width as usize * 4;
    let row_bytes = max_w as usize * 4;
    for row in 0..max_h as usize {
        let start = (y as usize + row) * stride + x as usize * 4;
        let end = start + row_bytes;
        if end > fb.pixels.len() {
            break;
        }
        // Fill 4 bytes at a time using chunks_exact_mut so the
        // bounds check happens once per row, not per pixel.
        for px in fb.pixels[start..end].chunks_exact_mut(4) {
            px[0] = color[0];
            px[1] = color[1];
            px[2] = color[2];
            px[3] = color[3];
        }
    }
}