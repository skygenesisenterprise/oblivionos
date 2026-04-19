use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    Applications,
    Settings,
}

#[derive(Debug, Clone)]
pub struct AppResult {
    pub id: String,
    pub name: String,
    pub command: String,
}

impl AppResult {
    pub fn new(id: String, name: String, command: String) -> Self {
        Self { id, name, command }
    }
}

#[derive(Debug, Clone)]
pub enum SearchResult {
    App(AppResult),
    Command(String),
}

pub struct AppFinder {
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub max_results: usize,
}

impl AppFinder {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            selected_index: 0,
            max_results: 10,
        }
    }

    pub fn search(&mut self, _query: &str) {
        self.results.clear();
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.results.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }
}

impl Default for AppFinder {
    fn default() -> Self {
        Self::new()
    }
}