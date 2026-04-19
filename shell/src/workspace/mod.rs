use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceId(u32);

impl WorkspaceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub windows: Vec<String>,
    pub active: bool,
}

impl Workspace {
    pub fn new(id: WorkspaceId, name: String) -> Self {
        Self {
            id,
            name,
            windows: Vec::new(),
            active: false,
        }
    }

    pub fn add_window(&mut self, window_id: String) {
        if !self.windows.contains(&window_id) {
            self.windows.push(window_id);
        }
    }

    pub fn remove_window(&mut self, window_id: &str) {
        self.windows.retain(|w| w != window_id);
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

pub struct WorkspaceManager {
    pub workspaces: HashMap<WorkspaceId, Workspace>,
    pub active_workspace: WorkspaceId,
    pub next_id: u32,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let mut manager = Self {
            workspaces: HashMap::new(),
            active_workspace: WorkspaceId(1),
            next_id: 1,
        };

        manager.add_workspace("Main".to_string());
        manager
    }

    pub fn add_workspace(&mut self, name: String) -> WorkspaceId {
        let id = WorkspaceId(self.next_id);
        self.next_id += 1;

        let workspace = Workspace::new(id, name);
        self.workspaces.insert(id, workspace);
        id
    }

    pub fn remove_workspace(&mut self, id: WorkspaceId) {
        if self.workspaces.len() > 1 {
            self.workspaces.remove(&id);
            if self.active_workspace == id {
                self.active_workspace = WorkspaceId(1);
            }
        }
    }

    pub fn get_workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(&id)
    }

    pub fn get_active_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(&self.active_workspace)
    }

    pub fn switch_to(&mut self, id: WorkspaceId) {
        if let Some(workspace) = self.workspaces.get_mut(&self.active_workspace) {
            workspace.set_active(false);
        }

        self.active_workspace = id;

        if let Some(workspace) = self.workspaces.get_mut(&id) {
            workspace.set_active(true);
        }
    }

    pub fn move_window_to(&mut self, window_id: &str, workspace_id: WorkspaceId) {
        for workspace in self.workspaces.values_mut() {
            workspace.remove_window(window_id);
        }

        if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
            workspace.add_window(window_id.to_string());
        }
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}