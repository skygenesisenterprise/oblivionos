use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DockItem {
    pub id: String,
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub command: String,
    pub running: bool,
    pub pinned: bool,
    pub notification_badge: Option<String>,
}

impl DockItem {
    pub fn new(id: String, name: String, command: String) -> Self {
        Self {
            id,
            name,
            icon_path: None,
            command,
            running: false,
            pinned: false,
            notification_badge: None,
        }
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPosition {
    Bottom,
    Left,
    Right,
}

impl Default for DockPosition {
    fn default() -> Self {
        Self::Bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSize {
    Small,
    Medium,
    Large,
}

impl Default for DockSize {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone)]
pub struct DockConfig {
    pub position: DockPosition,
    pub size: DockSize,
    pub magnification: bool,
    pub icon_size: u32,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            position: DockPosition::Bottom,
            size: DockSize::Medium,
            magnification: true,
            icon_size: 48,
        }
    }
}

pub struct Dock {
    pub items: Vec<DockItem>,
    pub config: DockConfig,
    pub hovered_index: Option<usize>,
}

impl Dock {
    pub fn new() -> anyhow::Result<Self> {
        let items = vec![
            DockItem::new("filemanager".to_string(), "Files".to_string(), "oblivion-filemanager".to_string()),
            DockItem::new("terminal".to_string(), "Terminal".to_string(), "oblivion-terminal".to_string()),
            DockItem::new("editor".to_string(), "Editor".to_string(), "oblivion-editor".to_string()),
            DockItem::new("settings".to_string(), "Settings".to_string(), "oblivion-settings".to_string()),
        ];

        Ok(Self {
            items,
            config: DockConfig::default(),
            hovered_index: None,
        })
    }

    pub fn add_item(&mut self, item: DockItem) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }

    pub fn get_item(&self, id: &str) -> Option<&DockItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn get_item_mut(&mut self, id: &str) -> Option<&mut DockItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }
}

impl Default for Dock {
    fn default() -> Self {
        Self::new().expect("Failed to create dock")
    }
}