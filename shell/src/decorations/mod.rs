#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationStyle {
    None,
    Client,
    Server,
}

impl Default for DecorationStyle {
    fn default() -> Self {
        Self::Server
    }
}

#[derive(Debug, Clone)]
pub struct DecorationColors {
    pub background: [u8; 4],
    pub title: [u8; 4],
    pub title_inactive: [u8; 4],
    pub button_close: [u8; 4],
    pub button_minimize: [u8; 4],
    pub button_maximize: [u8; 4],
}

impl Default for DecorationColors {
    fn default() -> Self {
        Self {
            background: [50, 50, 50, 200],
            title: [255, 255, 255, 255],
            title_inactive: [180, 180, 180, 255],
            button_close: [255, 95, 86, 255],
            button_minimize: [255, 190, 40, 255],
            button_maximize: [40, 205, 65, 255],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecorationConfig {
    pub style: DecorationStyle,
    pub height: u32,
    pub border_radius: (u32, u32, u32, u32),
    pub title_font_size: u32,
    pub colors: DecorationColors,
}

impl Default for DecorationConfig {
    fn default() -> Self {
        Self {
            style: DecorationStyle::Server,
            height: 38,
            border_radius: (12, 12, 0, 0),
            title_font_size: 13,
            colors: DecorationColors::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLightPosition {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficLight {
    Close,
    Minimize,
    Maximize,
}

#[derive(Debug, Clone)]
pub struct WindowDecorations {
    pub config: DecorationConfig,
    pub hovered_button: Option<TrafficLight>,
}

impl WindowDecorations {
    pub fn new() -> Self {
        Self {
            config: DecorationConfig::default(),
            hovered_button: None,
        }
    }

    pub fn set_hovered_button(&mut self, button: Option<TrafficLight>) {
        self.hovered_button = button;
    }
}

impl Default for WindowDecorations {
    fn default() -> Self {
        Self::new()
    }
}