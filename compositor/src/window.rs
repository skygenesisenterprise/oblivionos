use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(u32);

impl WindowId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
    Floating,
    Hidden,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRole {
    Normal,
    Dialog,
    Modal,
    Splash,
    Utility,
    Dock,
    Toolbar,
    Menu,
    Popup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDecoration {
    None,
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabMode {
    None,
    Moving,
    Resizing,
    EdgeResizing,
    Keyboard,
}

#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub state: WindowState,
    pub role: WindowRole,
    pub decoration: WindowDecoration,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub saved_x: Option<i32>,
    pub saved_y: Option<i32>,
    pub saved_width: Option<u32>,
    pub saved_height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub z_index: u32,
    pub stack_index: u32,
    pub visible: bool,
    pub focused: bool,
    pub urgent: bool,
    pub modal: bool,
    pub pinned: bool,
    pub shadow: bool,
    pub opacity: f32,
    pub grab_mode: GrabMode,
    pub grab_offset: Option<(i32, i32)>,
    pub grab_edge: Option<ResizeEdge>,
    pub created_at: Instant,
    pub focused_at: Option<Instant>,
    pub last_configure: Option<Instant>,
    pub pid: Option<u32>,
    pub wm_class: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Window {
    pub fn new(id: WindowId, title: String, app_id: String) -> Self {
        Self {
            id,
            title,
            app_id,
            state: WindowState::Normal,
            role: WindowRole::Normal,
            decoration: WindowDecoration::Server,
            x: 100,
            y: 100,
            width: 800,
            height: 600,
            saved_x: None,
            saved_y: None,
            saved_width: None,
            saved_height: None,
            min_width: Some(400),
            min_height: Some(300),
            max_width: None,
            max_height: None,
            z_index: 0,
            stack_index: 0,
            visible: true,
            focused: false,
            urgent: false,
            modal: false,
            pinned: false,
            shadow: true,
            opacity: 1.0,
            grab_mode: GrabMode::None,
            grab_offset: None,
            grab_edge: None,
            created_at: Instant::now(),
            focused_at: None,
            last_configure: None,
            pid: None,
            wm_class: String::new(),
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        let width = self.clamp_width(width);
        let height = self.clamp_height(height);
        self.width = width;
        self.height = height;
    }

    fn clamp_width(&self, mut width: u32) -> u32 {
        if let Some(min) = self.min_width {
            width = width.max(min);
        }
        if let Some(max) = self.max_width {
            width = width.min(max);
        }
        width
    }

    fn clamp_height(&self, mut height: u32) -> u32 {
        if let Some(min) = self.min_height {
            height = height.max(min);
        }
        if let Some(max) = self.max_height {
            height = height.min(max);
        }
        height
    }

    pub fn activate(&mut self) {
        self.state = WindowState::Normal;
        self.focused = true;
        self.urgent = false;
        self.focused_at = Some(Instant::now());
    }

    pub fn minimize(&mut self) {
        self.state = WindowState::Minimized;
        self.focused = false;
    }

    pub fn maximize(&mut self) {
        if self.state != WindowState::Maximized {
            self.saved_x = Some(self.x);
            self.saved_y = Some(self.y);
            self.saved_width = Some(self.width);
            self.saved_height = Some(self.height);
        }
        self.state = WindowState::Maximized;
    }

    pub fn restore(&mut self) {
        if matches!(self.state, WindowState::Maximized | WindowState::Fullscreen) {
            if let (Some(x), Some(y), Some(width), Some(height)) = 
                (self.saved_x, self.saved_y, self.saved_width, self.saved_height) 
            {
                self.x = x;
                self.y = y;
                self.width = width;
                self.height = height;
            }
        }
        self.state = WindowState::Normal;
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if fullscreen && self.state != WindowState::Fullscreen {
            self.saved_x = Some(self.x);
            self.saved_y = Some(self.y);
            self.saved_width = Some(self.width);
            self.saved_height = Some(self.height);
            self.state = WindowState::Fullscreen;
        } else if !fullscreen && self.state == WindowState::Fullscreen {
            self.restore();
        }
    }

    pub fn start_drag(&mut self, offset_x: i32, offset_y: i32) {
        self.grab_mode = GrabMode::Moving;
        self.grab_offset = Some((offset_x, offset_y));
    }

    pub fn start_resize(&mut self, edge: ResizeEdge, offset_x: i32, offset_y: i32) {
        self.grab_mode = match edge {
            ResizeEdge::Left | ResizeEdge::Right => GrabMode::EdgeResizing,
            _ => GrabMode::Resizing,
        };
        self.grab_edge = Some(edge);
        self.grab_offset = Some((offset_x, offset_y));
    }

    pub fn end_grab(&mut self) {
        self.grab_mode = GrabMode::None;
        self.grab_edge = None;
        self.grab_offset = None;
    }

    pub fn is_resizing(&self) -> bool {
        matches!(self.grab_mode, GrabMode::Resizing | GrabMode::EdgeResizing)
    }

    pub fn is_moving(&self) -> bool {
        matches!(self.grab_mode, GrabMode::Moving)
    }

    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width as i32 &&
        py >= self.y && py < self.y + self.height as i32
    }
}

pub struct WindowManager {
    windows: HashMap<WindowId, Window>,
    next_id: u32,
    focused_window: Option<WindowId>,
    active_stack: Vec<WindowId>,
    z_index_counter: u32,
    awaiting_close: Vec<WindowId>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            next_id: 1,
            focused_window: None,
            active_stack: Vec::new(),
            z_index_counter: 0,
            awaiting_close: Vec::new(),
        }
    }

    pub fn create_window(&mut self, title: String, app_id: String) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;

        let mut window = Window::new(id, title, app_id);
        window.z_index = self.z_index_counter;
        self.z_index_counter += 1;

        self.windows.insert(id, window);
        self.active_stack.push(id);
        self.focus_window(id);

        tracing::debug!("Created window: {:?}", id);
        id
    }

    pub fn destroy_window(&mut self, id: WindowId) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.state = WindowState::Closing;
            self.awaiting_close.push(id);
        }
    }

    pub fn confirm_destroy(&mut self, id: WindowId) {
        self.windows.remove(&id);
        self.active_stack.retain(|&w| w != id);
        
        if self.focused_window == Some(id) {
            self.focused_window = self.active_stack.last().copied();
        }

        tracing::debug!("Destroyed window: {:?}", id);
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn focus_window(&mut self, id: WindowId) {
        if let Some(focused) = self.focused_window {
            if let Some(window) = self.windows.get_mut(&focused) {
                window.focused = false;
            }
        }

        self.focused_window = Some(id);

        if let Some(window) = self.windows.get_mut(&id) {
            window.focused = true;
            window.activate();
            window.z_index = self.z_index_counter;
            self.z_index_counter += 1;

            if let Some(idx) = self.active_stack.iter().position(|&w| w == id) {
                self.active_stack.remove(idx);
                self.active_stack.push(id);
            }
        }
    }

    pub fn get_focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }

    pub fn raise_window(&mut self, id: WindowId) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.z_index = self.z_index_counter;
            self.z_index_counter += 1;
        }
    }

    pub fn move_window(&mut self, id: WindowId, x: i32, y: i32) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.set_position(x, y);
        }
    }

    pub fn resize_window(&mut self, id: WindowId, width: u32, height: u32) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.set_size(width, height);
        }
    }

    pub fn get_windows_at_point(&self, x: i32, y: i32) -> Vec<WindowId> {
        let mut found: Vec<_> = self.windows
            .values()
            .filter(|w| w.visible && w.contains_point(x, y))
            .map(|w| w.id)
            .collect();

        found.sort_by(|&a, &b| {
            let wa = self.windows.get(&a).unwrap();
            let wb = self.windows.get(&b).unwrap();
            wb.z_index.cmp(&wa.z_index)
        });

        found
    }

    pub fn get_all_windows(&self) -> Vec<&Window> {
        self.windows.values().collect()
    }

    pub fn get_visible_windows(&self) -> Vec<&Window> {
        self.windows
            .values()
            .filter(|w| w.visible && w.state != WindowState::Minimized)
            .collect()
    }

    pub fn get_stacked_windows(&self) -> Vec<&Window> {
        let mut windows: Vec<_> = self.get_visible_windows();
        windows.sort_by(|a, b| a.z_index.cmp(&b.z_index));
        windows
    }

    pub fn get_top_window(&self) -> Option<&Window> {
        self.get_stacked_windows().last().map(|w| *w)
    }

    pub fn get_closing_windows(&self) -> Vec<WindowId> {
        self.awaiting_close.clone()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn has_visible_windows(&self) -> bool {
        !self.get_visible_windows().is_empty()
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}