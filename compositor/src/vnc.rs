//! compositor/src/vnc.rs
//!
//! Stage 3 of the VNC roadmap: in-process RFB/VNC server inside the
//! OblivionOS compositor. Speaks RFB protocol version 3.8 with
//! `Security = None` and the mandatory `Raw` encoding.
//!
//! Lifecycle:
//!   - `Compositor::with_input_channel(...)` builds a framebuffer + an
//!     mpsc receiver for `InputEvent`s. The main thread ticks the
//!     framebuffer and drains input events in its 16 ms loop.
//!   - If `OBLIVION_VNC=1` is set, `main` additionally spawns
//!     `vnc::run_server(...)` on tokio, which owns a clone of the
//!     framebuffer `Arc<RwLock<...>>` and a sender into the same channel.
//!
//! Roadmap / known limits (intentional for the scaffold):
//!   * only `Raw` rects are produced. When the real renderer lands,
//!     splatting BGRA from a damage region is a single `memcpy`.
//!   * no DES / Unix login auth; tighten before exposing on LAN.
//!   * no ZRLE/Hextile; the framebuffer is small enough that Raw is fine.
//!   * input events go in via a `mpsc::Sender<InputEvent>`. They are
//!     consumed by `Compositor::run` via `try_recv` and queued into
//!     `compositor::input::InputHandler`. When the Wayland renderer is
//!     wired we just call `input_handler.queue_event(...)` from the same
//!     channel path — no duplication.

use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::render::{DamageState, Rect};

/// Maximum width/height per FramebufferUpdateRequest rect. The protocol
/// itself allows up to 65535, but allocating a `Vec<u8>` of
/// `w * h * 4` bytes for a hostile client would OOM the process; clamp
/// here and let the client ask for more (smaller) rects as it wishes.
const MAX_RECT_DIM: u16 = 4096;

/// RFB pseudo-encoding that terminates an incremental update: a rect
/// whose `width = height = 0` and `encoding = -224`. Clients reading a
/// `FramebufferUpdate` know to stop expecting rects after one of these.
const ENC_LAST_RECT: i32 = -224;

use crate::input::{
    ButtonState, InputDeviceId, InputEvent, InputEventType, KeyState, ModifiersState,
};

pub const DEFAULT_WIDTH: u16 = 1024;
pub const DEFAULT_HEIGHT: u16 = 768;

/// RGBX8 packed framebuffer. Stored as 4 bytes per pixel in `[R, G, B, _pad]`
/// order — when the server announces `ServerInit` with `big-endian-flag=1`,
/// `red-shift=24`, `green-shift=16`, `blue-shift=8`, the bytes we send on
/// the wire as the framebuffer data are interpreted by the client exactly
/// as `[R, G, B, _]` (one BGRA-compatible rect = one VNC pixel).
#[derive(Debug)]
pub struct Framebuffer {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub sequence: u64,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16, fill: [u8; 4]) -> Self {
        let mut fb = Self {
            width,
            height,
            pixels: vec![0u8; width as usize * height as usize * 4],
            sequence: 0,
        };
        fb.fill_solid(fill);
        fb
    }

    pub fn fill_solid(&mut self, color: [u8; 4]) {
        let px = [color[0], color[1], color[2], color[3]];
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&px);
        }
        self.sequence = self.sequence.wrapping_add(1);
    }

    pub fn gradient(&mut self, top: [u8; 4], bottom: [u8; 4]) {
        let h = self.height as usize;
        if h == 0 {
            return;
        }
        let w = self.width as usize;
        for y in 0..h {
            let t = y as f32 / (h - 1) as f32;
            let r = lerp(top[0], bottom[0], t);
            let g = lerp(top[1], bottom[1], t);
            let b = lerp(top[2], bottom[2], t);
            let a = lerp(top[3], bottom[3], t);
            let row_start = y * w * 4;
            let row_end = row_start + w * 4;
            for chunk in self.pixels[row_start..row_end].chunks_exact_mut(4) {
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = a;
            }
        }
        self.sequence = self.sequence.wrapping_add(1);
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 * (1.0 - t) + b as f32 * t) as u8
}

#[derive(Clone)]
pub struct VncConfig {
    pub bind_addr: String,
    pub name: String,
    pub fb: Arc<RwLock<Framebuffer>>,
    pub input_tx: mpsc::Sender<InputEvent>,
    /// Shared damage state so VNC clients receive the same rect list
    /// the compositor publishes. Always set, even when no client is
    /// connected — the compositor publishes every paint.
    pub damage: Arc<DamageState>,
}

pub async fn run_server(cfg: VncConfig) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&cfg.bind_addr).await?;
    let local = listener.local_addr()?;
    info!(addr = %local, "VNC server listening (RFB 3.8, Security=None, Raw only)");
    loop {
        let (stream, peer) = listener.accept().await?;
        debug!(%peer, "VNC client connected");
        let client_cfg = cfg.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, client_cfg, peer).await {
                warn!(%peer, "VNC client disconnected: {e:#}");
            }
        });
    }
}

async fn handle_client(
    mut s: TcpStream,
    cfg: VncConfig,
    peer: std::net::SocketAddr,
) -> anyhow::Result<()> {
    // ---- Handshake: server's protocol version ----------------
    s.write_all(b"RFB 003.008\n").await?;
    // ---- Client's protocol version echo ----------------------
    let mut version = [0u8; 12];
    s.read_exact(&mut version).await?;
    debug!(%peer, ?version, "client protocol version");

    // ---- Security: server picks "None" only ------------------
    // 1 byte: number of types; then 'count' bytes of types.
    s.write_all(&[1u8, 1u8]).await?;
    // 1 byte: client selects None (=1)
    let mut chosen = [0u8; 1];
    s.read_exact(&mut chosen).await?;
    if chosen[0] != 1 {
        anyhow::bail!("client chose unsupported security {}", chosen[0]);
    }
    // SecurityResult: u32 BE (0 == OK)
    s.write_all(&0u32.to_be_bytes()).await?;

    // ---- ClientInit: 1 byte shared-flag + u16 BE name len + name bytes
    let mut shared = [0u8; 1];
    s.read_exact(&mut shared).await?;
    let mut name_len_bytes = [0u8; 2];
    s.read_exact(&mut name_len_bytes).await?;
    let name_len = u16::from_be_bytes(name_len_bytes);
    let mut client_name = vec![0u8; name_len as usize];
    s.read_exact(&mut client_name).await?;
    debug!(
        %peer,
        shared = shared[0] != 0,
        client_name = String::from_utf8_lossy(&client_name).as_ref(),
        "ClientInit"
    );

    // ---- ServerInit: width (u16 BE) + height (u16 BE) + 16 bytes pixel format
    let (w, h) = {
        let fb = cfg.fb.read();
        (fb.width, fb.height)
    };
    s.write_all(&w.to_be_bytes()).await?;
    s.write_all(&h.to_be_bytes()).await?;
    // 16-byte pixel format. big-endian-flag=1 means every multi-byte value
    // we send on the wire is big-endian. R/G/B shifts of 24/16/8 mean a
    // 32-bit pixel value laid out in memory as [R, G, B, _pad] works
    // directly frame-to-wire (no byte-swap gymnastics).
    let px_format: [u8; 16] = [
        32,    // bpp
        24,    // depth
        1,     // big-endian-flag
        1,     // true-color
        0x00, 0xFF, // red-max
        0x00, 0xFF, // green-max
        0x00, 0xFF, // blue-max
        24,    // red-shift
        16,    // green-shift
        8,     // blue-shift
        0, 0, 0, // padding
    ];
    s.write_all(&px_format).await?;
    let name_bytes = cfg.name.as_bytes();
    s.write_all(&(name_bytes.len() as u32).to_be_bytes()).await?;
    s.write_all(name_bytes).await?;
    s.flush().await?;

    // ---- Message loop ---------------------------------------
    // `last_pointer_mask` tracks which mouse buttons were down on the
    // previous PointerEvent. VNC clients only send the *current* button
    // mask each event, so we have to infer pressed/released transitions
    // ourselves or InputHandler would see "left button permanently held
    // down" the moment the user clicks once.
    let mut last_pointer_mask: u8 = 0;
    loop {
        let mut msg_type = [0u8; 1];
        match s.read_exact(&mut msg_type).await {
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
        match msg_type[0] {
            0 => {
                // SetPixelFormat: 3-byte pad + 16-byte pixel format + 3-byte pad
                let mut buf = [0u8; 3 + 16 + 3];
                s.read_exact(&mut buf).await?;
                debug!(%peer, "SetPixelFormat accepted (we ignore and stay server-formatted)");
            }
            2 => {
                // SetEncodings: 1-byte pad + u16 BE count + count*i32 BE
                // Per RFB 3.8 §7.5.2, the count is BE-on-wire because
                // big-endian-flag is 1 in our ServerInit. tokio's
                // `read_u16()` reads host-native (LE on x86_64), so we
                // must parse explicitly to avoid producing a 16-bit value
                // that disagrees with the client.
                let mut pad = [0u8; 1];
                s.read_exact(&mut pad).await?;
                let mut count_bytes = [0u8; 2];
                s.read_exact(&mut count_bytes).await?;
                let count = u16::from_be_bytes(count_bytes) as usize;
                let mut enc_buf = vec![0u8; count * 4];
                s.read_exact(&mut enc_buf).await?;
                debug!(%peer, encodings = count, "SetEncodings");
            }
            3 => {
                // FramebufferUpdateRequest: u8 incremental + u16 BE x + y + w + h
                let mut buf = [0u8; 9];
                s.read_exact(&mut buf).await?;
                let incremental = buf[0] != 0;
                let x = u16::from_be_bytes([buf[1], buf[2]]);
                let y = u16::from_be_bytes([buf[3], buf[4]]);
                let w = u16::from_be_bytes([buf[5], buf[6]]);
                let h = u16::from_be_bytes([buf[7], buf[8]]);
                let (full_w, full_h) = {
                    let fb = cfg.fb.read();
                    (fb.width, fb.height)
                };
                debug!(
                    %peer, incremental, x, y, w, h,
                    "FramebufferUpdateRequest"
                );

                if !incremental {
                    // Non-incremental request: client wants the requested
                    // region in full, regardless of damage state. Drain
                    // pending damage (we're claiming those rects are now
                    // sent) and ship one Raw rect for the requested area
                    // followed by a LastRect terminator.
                    let _ = cfg.damage.drain();
                    let cx = x.min(full_w.saturating_sub(1));
                    let cy = y.min(full_h.saturating_sub(1));
                    let cw = w.min(MAX_RECT_DIM).min(full_w - cx).max(1);
                    let ch = h.min(MAX_RECT_DIM).min(full_h - cy).max(1);

                    // FBUPDATE header (msg-type + padding + rect-count=1).
                    s.write_all(&[0u8, 0u8]).await?;
                    s.write_all(&1u16.to_be_bytes()).await?;
                    send_raw_rect(&mut s, cx, cy, cw, ch, cfg.fb.clone()).await?;
                    send_last_rect_marker(&mut s).await?;
                    s.flush().await?;
                } else {
                    // Incremental request: wait (with a 1-second ceiling)
                    // for the compositor to publish damage, then ship
                    // the queued dirty rects. The notified() future MUST
                    // be created BEFORE the first peek, otherwise we can
                    // miss a notify that arrives between peek and wait.
                    let mut dirty = cfg.damage.peek();
                    if dirty.is_empty() {
                        let notified = cfg.damage.notify.notified();
                        match tokio::time::timeout(std::time::Duration::from_secs(1), notified).await {
                            Ok(()) => dirty = cfg.damage.drain(),
                            Err(_) => {
                                debug!(%peer, "incremental: 1s timeout, sending empty update");
                                dirty.clear();
                            }
                        }
                    } else {
                        // Take ownership of the rects we just peeked.
                        dirty = cfg.damage.drain();
                    }

                    // Even with 0 dirty rects, send an empty FBUPDATE so
                    // a client that's sitting on a non-incremental loop
                    // sees the server is still alive. Empty == "no
                    // changes", not "disconnected".
                    let rect_count = dirty.len() as u16;
                    debug!(%peer, rect_count, "incremental: sending dirty rects");
                    s.write_all(&[0u8, 0u8]).await?;
                    s.write_all(&rect_count.to_be_bytes()).await?;
                    for r in &dirty {
                        let cx = r.x.max(0).min(full_w as i32 - 1).max(0) as u16;
                        let cy = r.y.max(0).min(full_h as i32 - 1).max(0) as u16;
                        let cw = r.w.min(MAX_RECT_DIM as u32) as u16;
                        let ch = r.h.min(MAX_RECT_DIM as u32) as u16;
                        if cw == 0 || ch == 0 {
                            continue;
                        }
                        send_raw_rect(&mut s, cx, cy, cw, ch, cfg.fb.clone()).await?;
                    }
                    if rect_count > 0 {
                        send_last_rect_marker(&mut s).await?;
                    }
                    s.flush().await?;
                }
            }
            4 => {
                // KeyEvent: u8 down + u16 pad + u32 BE keysym
                let mut buf = [0u8; 7];
                s.read_exact(&mut buf).await?;
                let down = buf[0] != 0;
                let key = u32::from_be_bytes([buf[3], buf[4], buf[5], buf[6]]);
                let _ = cfg.input_tx.send(InputEvent {
                    timestamp: Instant::now(),
                    device_id: InputDeviceId::new(0),
                    event_type: InputEventType::KeyPress {
                        key,
                        state: if down { KeyState::Pressed } else { KeyState::Released },
                        modifiers: ModifiersState::default(),
                    },
                }).await;
            }
            5 => {
                // PointerEvent: u8 button-mask + u16 BE x + y
                let mut buf = [0u8; 5];
                s.read_exact(&mut buf).await?;
                let mask = buf[0];
                let x = u16::from_be_bytes([buf[1], buf[2]]) as f64;
                let y = u16::from_be_bytes([buf[3], buf[4]]) as f64;
                let _ = cfg.input_tx.send(InputEvent {
                    timestamp: Instant::now(),
                    device_id: InputDeviceId::new(0),
                    event_type: InputEventType::MouseMotion { x, y, dx: 0.0, dy: 0.0 },
                }).await;
                // Emit a Pressed/Released event for any of the eight RFB
                // mouse-button bits that flipped between this frame and
                // the previous one (bits 0..7 => buttons 1..8 per
                // RFC 6143 §7.5.5). The bit pattern is the protocol's
                // *current* mask — we synthesize transitions ourselves so
                // InputHandler doesn't see e.g. "left button permanently
                // held down" after a single click.
                //
                // IMPORTANT: RFB does NOT encode scroll-wheel motion.
                // Bits 3 (0x08) and 4 (0x10) are buttons 4 and 5, NOT
                // scroll-up/down. Anything that wants wheel events on a
                // VNC client needs a vendor extension (xvp / fbs / etc.).
                for bit in 0u8..8 {
                    let was = (last_pointer_mask >> bit) & 1;
                    let is = (mask >> bit) & 1;
                    if was != is {
                        let _ = cfg.input_tx.send(InputEvent {
                            timestamp: Instant::now(),
                            device_id: InputDeviceId::new(0),
                            event_type: InputEventType::MouseButton {
                                button: bit as u32 + 1,
                                state: if is != 0 {
                                    ButtonState::Pressed
                                } else {
                                    ButtonState::Released
                                },
                            },
                        }).await;
                    }
                }
                last_pointer_mask = mask;
            }
            6 => {
                // ClientCutText: 3-byte pad + u32 BE length + N bytes
                let mut pad = [0u8; 3];
                s.read_exact(&mut pad).await?;
                let mut len_bytes = [0u8; 4];
                s.read_exact(&mut len_bytes).await?;
                let len = u32::from_be_bytes(len_bytes) as usize;
                let mut text = vec![0u8; len];
                s.read_exact(&mut text).await?;
                debug!(%peer, len, "ClientCutText");
            }
            other => {
                anyhow::bail!("unknown msg-type {other}");
            }
        }
    }
}

/// Send the RFB `LastRect` pseudo-encoding terminator. Per RFC 6143
/// §7.6.1, an update containing one or more real rects is closed by
/// one more rect with x=0, y=0, width=0, height=0, encoding=-224.
async fn send_last_rect_marker(s: &mut TcpStream) -> anyhow::Result<()> {
    s.write_all(&[0u8, 0u8]).await?;       // x, y
    s.write_all(&[0u8, 0u8]).await?;       // width, height
    s.write_all(&ENC_LAST_RECT.to_be_bytes()).await?;
    Ok(())
}

/// Send one Big-Raw rectangle to the client. To avoid blocking the
/// compositor's render thread on slow TCP, we snapshot the pixel rows
/// into a local `Vec<u8>` *inside* the read-lock, then drop the lock
/// before doing the actual TCP writes.
async fn send_raw_rect(
    s: &mut TcpStream,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    fb_lock: Arc<RwLock<Framebuffer>>,
) -> anyhow::Result<()> {
    let snapshot: Vec<u8> = {
        let fb = fb_lock.read();
        let stride = fb.width as usize * 4;
        let mut buf = Vec::with_capacity(w as usize * h as usize * 4);
        for row in 0..(h as usize) {
            let offset = (y as usize + row) * stride + (x as usize) * 4;
            let end = offset + w as usize * 4;
            if end > fb.pixels.len() {
                anyhow::bail!("framebuffer slice {offset}+{} out of bounds", end - offset);
            }
            buf.extend_from_slice(&fb.pixels[offset..end]);
        }
        buf
    };

    // RFC 6143 §7.6.1: FramebufferUpdate header is
    //     u8   message-type   = 0
    //     u8   padding        = 0
    //     u16  rect-count     (BE)
    //     ... rect headers + data
    // The padding byte was missing in r0 of this file and produced a one-
    // byte wire desync where clients parsed rect-count as 256 instead of 1.
    s.write_all(&[0u8, 0u8]).await?;
    s.write_all(&1u16.to_be_bytes()).await?;
    // per-rect: x(2) + y(2) + w(2) + h(2) + encoding(4 = Raw = 0)
    let mut hdr = Vec::with_capacity(12);
    hdr.extend_from_slice(&x.to_be_bytes());
    hdr.extend_from_slice(&y.to_be_bytes());
    hdr.extend_from_slice(&w.to_be_bytes());
    hdr.extend_from_slice(&h.to_be_bytes());
    hdr.extend_from_slice(&0i32.to_be_bytes());
    s.write_all(&hdr).await?;
    s.write_all(&snapshot).await?;
    s.flush().await?;
    Ok(())
}
