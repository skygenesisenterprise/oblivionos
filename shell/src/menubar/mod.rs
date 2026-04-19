use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarPosition {
    Top,
    Bottom,
}

impl Default for MenuBarPosition {
    fn default() -> Self {
        Self::Top
    }
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

impl MenuItem {
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            shortcut: None,
            enabled: true,
        }
    }

    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub id: String,
    pub label: String,
    pub items: Vec<MenuItem>,
    pub open: bool,
}

impl Menu {
    pub fn new(id: String, label: String) -> Self {
        Self {
            id,
            label,
            items: Vec::new(),
            open: false,
        }
    }

    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

#[derive(Debug, Clone)]
pub struct StatusIcon {
    pub id: String,
    pub tooltip: String,
    pub icon_path: Option<String>,
}

impl StatusIcon {
    pub fn new(id: String, tooltip: String) -> Self {
        Self {
            id,
            tooltip,
            icon_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClockState {
    pub hour_format_12h: bool,
    pub show_date: bool,
    pub show_seconds: bool,
}

impl ClockState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct MenuBar {
    pub position: MenuBarPosition,
    pub menus: Vec<Menu>,
    pub status_icons: HashMap<String, StatusIcon>,
    pub clock: ClockState,
    pub focused_menu: Option<String>,
}

impl MenuBar {
    pub fn new() -> anyhow::Result<Self> {
        let mut menus = Vec::new();

        let mut apple = Menu::new("apple".to_string(), "OblivionOS".to_string());
        apple.add_item(MenuItem::new("about".to_string(), "About OblivionOS".to_string()));
        apple.add_item(MenuItem::new("quit".to_string(), "Quit".to_string()));
        menus.push(apple);

        menus.push(Menu::new("file".to_string(), "File".to_string()));

        let status_icons = HashMap::new();

        Ok(Self {
            position: MenuBarPosition::Top,
            menus,
            status_icons,
            clock: ClockState::default(),
            focused_menu: None,
        })
    }

    pub fn add_menu(&mut self, menu: Menu) {
        self.menus.push(menu);
    }

    pub fn get_menu(&self, id: &str) -> Option<&Menu> {
        self.menus.iter().find(|m| m.id == id)
    }

    pub fn format_time(&self) -> String {
        "00:00".to_string()
    }

    pub fn format_date(&self) -> String {
        "Mon Jan 1".to_string()
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new().expect("Failed to create menu bar")
    }
}