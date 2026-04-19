use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XdgToplevelId(u32);

impl XdgToplevelId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XdgPopupId(u32);

impl XdgPopupId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToplevelState {
    Activated,
    Deactivated,
    Maximized,
    Fullscreen,
    Minimized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupState {
    Shown,
    Dismissed,
}

#[derive(Debug, Clone)]
pub struct XdgToplevel {
    pub id: XdgToplevelId,
    pub title: String,
    pub app_id: String,
    pub state: ToplevelState,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub decorations: bool,
    pub resizable: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub active: bool,
    pub urgent: bool,
}

impl XdgToplevel {
    pub fn new(id: XdgToplevelId, title: String, app_id: String) -> Self {
        Self {
            id,
            title,
            app_id,
            state: ToplevelState::Activated,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            decorations: true,
            resizable: true,
            fullscreen: false,
            maximized: false,
            minimized: false,
            active: true,
            urgent: false,
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
        if fullscreen { self.maximized = false; }
    }

    pub fn set_maximized(&mut self, maximized: bool) {
        self.maximized = maximized;
        if maximized { self.fullscreen = false; }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active { self.urgent = false; }
    }
}

#[derive(Debug, Clone)]
pub struct XdgPopup {
    pub id: XdgPopupId,
    pub state: PopupState,
    pub parent: Option<XdgToplevelId>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl XdgPopup {
    pub fn new(id: XdgPopupId) -> Self {
        Self {
            id,
            state: PopupState::Dismissed,
            parent: None,
            x: 0,
            y: 0,
            width: 300,
            height: 200,
        }
    }

    pub fn show(&mut self) {
        self.state = PopupState::Shown;
    }

    pub fn dismiss(&mut self) {
        self.state = PopupState::Dismissed;
    }

    pub fn is_visible(&self) -> bool {
        matches!(self.state, PopupState::Shown)
    }
}

pub struct XdgManager {
    toplevels: HashMap<XdgToplevelId, XdgToplevel>,
    popups: HashMap<XdgPopupId, XdgPopup>,
    active_toplevel: Option<XdgToplevelId>,
    next_toplevel_id: u32,
    next_popup_id: u32,
}

impl XdgManager {
    pub fn new() -> Self {
        Self {
            toplevels: HashMap::new(),
            popups: HashMap::new(),
            active_toplevel: None,
            next_toplevel_id: 1,
            next_popup_id: 1,
        }
    }

    pub fn create_toplevel(&mut self, title: String, app_id: String) -> XdgToplevelId {
        let id = XdgToplevelId(self.next_toplevel_id);
        self.next_toplevel_id += 1;

        let toplevel = XdgToplevel::new(id, title, app_id);
        self.toplevels.insert(id, toplevel);
        id
    }

    pub fn create_popup(&mut self) -> XdgPopupId {
        let id = XdgPopupId(self.next_popup_id);
        self.next_popup_id += 1;

        let popup = XdgPopup::new(id);
        self.popups.insert(id, popup);
        id
    }

    pub fn get_toplevel(&self, id: XdgToplevelId) -> Option<&XdgToplevel> {
        self.toplevels.get(&id)
    }

    pub fn get_popup(&self, id: XdgPopupId) -> Option<&XdgPopup> {
        self.popups.get(&id)
    }

    pub fn destroy_toplevel(&mut self, id: XdgToplevelId) {
        if self.active_toplevel == Some(id) {
            self.active_toplevel = None;
        }
        self.toplevels.remove(&id);
    }

    pub fn set_active_toplevel(&mut self, id: Option<XdgToplevelId>) {
        if let Some(old) = self.active_toplevel {
            if let Some(toplevel) = self.toplevels.get_mut(&old) {
                toplevel.set_active(false);
            }
        }

        self.active_toplevel = id;

        if let Some(new) = id {
            if let Some(toplevel) = self.toplevels.get_mut(&new) {
                toplevel.set_active(true);
            }
        }
    }

    pub fn get_all_toplevels(&self) -> Vec<&XdgToplevel> {
        self.toplevels.values().collect()
    }
}

impl Default for XdgManager {
    fn default() -> Self {
        Self::new()
    }
}