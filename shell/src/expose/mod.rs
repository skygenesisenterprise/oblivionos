use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposeLayout {
    Grid,
    Fan,
    Stack,
}

impl Default for ExposeLayout {
    fn default() -> Self {
        Self::Grid
    }
}

#[derive(Debug, Clone)]
pub struct ExposedWindow {
    pub window_id: String,
    pub title: String,
    pub app_id: String,
}

impl ExposedWindow {
    pub fn new(window_id: String, title: String, app_id: String) -> Self {
        Self {
            window_id,
            title,
            app_id,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExposeState {
    pub active: bool,
    pub layout: ExposeLayout,
}

pub struct ExposeManager {
    pub state: ExposeState,
    pub config: ExposeConfig,
}

#[derive(Debug, Clone, Default)]
pub struct ExposeConfig {
    pub animation_duration_ms: u32,
    pub grid_columns: u32,
    pub grid_rows: u32,
}

impl ExposeManager {
    pub fn new() -> Self {
        Self {
            state: ExposeState::default(),
            config: ExposeConfig::default(),
        }
    }

    pub fn activate(&mut self) {
        self.state.active = true;
    }

    pub fn deactivate(&mut self) {
        self.state.active = false;
    }

    pub fn toggle(&mut self) {
        self.state.active = !self.state.active;
    }
}

impl Default for ExposeManager {
    fn default() -> Self {
        Self::new()
    }
}