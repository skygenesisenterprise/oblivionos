use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub focused: bool,
}

pub struct WindowManager {
    windows: HashMap<u32, Window>,
    next_id: u32,
    focused_window: Option<u32>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            next_id: 1,
            focused_window: None,
        }
    }

    pub fn create_window(&mut self, title: String, x: i32, y: i32, width: u32, height: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let window = Window {
            id,
            title,
            x,
            y,
            width,
            height,
            visible: true,
            focused: false,
        };

        self.windows.insert(id, window);
        self.focused_window = Some(id);

        id
    }

    pub fn destroy_window(&mut self, id: u32) {
        self.windows.remove(&id);
        if self.focused_window == Some(id) {
            self.focused_window = self.windows.keys().max().copied();
        }
    }

    pub fn get_window(&self, id: u32) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: u32) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn focus_window(&mut self, id: u32) {
        if let Some(focused) = self.focused_window {
            if let Some(window) = self.windows.get_mut(&focused) {
                window.focused = false;
            }
        }

        if let Some(window) = self.windows.get_mut(&id) {
            window.focused = true;
        }
        self.focused_window = Some(id);
    }

    pub fn move_window(&mut self, id: u32, x: i32, y: i32) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.x = x;
            window.y = y;
        }
    }

    pub fn resize_window(&mut self, id: u32, width: u32, height: u32) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.width = width;
            window.height = height;
        }
    }

    pub fn get_all_windows(&self) -> Vec<&Window> {
        self.windows.values().collect()
    }

    pub fn get_focused_window(&self) -> Option<u32> {
        self.focused_window
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
