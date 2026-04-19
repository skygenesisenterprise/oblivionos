#[derive(Debug, Clone)]
pub struct TrayItem {
    pub id: String,
    pub label: String,
    pub icon_path: Option<String>,
}

impl TrayItem {
    pub fn new(id: String) -> Self {
        Self {
            id,
            label: String::new(),
            icon_path: None,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = label;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct TrayMenuItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TrayMenu {
    pub items: Vec<TrayMenuItem>,
}

impl TrayMenu {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_item(&mut self, item: TrayMenuItem) {
        self.items.push(item);
    }
}

#[derive(Debug, Clone, Default)]
pub struct SystemTray {
    pub items: Vec<TrayItem>,
    pub hidden: bool,
}

impl SystemTray {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            hidden: false,
        }
    }

    pub fn add_item(&mut self, item: TrayItem) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }

    pub fn show(&mut self) {
        self.hidden = false;
    }

    pub fn hide(&mut self) {
        self.hidden = true;
    }
}