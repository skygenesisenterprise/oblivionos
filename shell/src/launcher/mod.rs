use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub command: String,
    pub icon: Option<PathBuf>,
}

impl AppEntry {
    pub fn new(id: String, name: String, command: String) -> Self {
        Self {
            id,
            name,
            command,
            icon: None,
        }
    }

    pub fn launch(&self) -> anyhow::Result<u32> {
        let child = Command::new(&self.command).spawn()?;
        tracing::info!("Launched: {} (pid: {})", self.name, child.id());
        Ok(child.id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Normal,
    Floating,
    Fullscreen,
}

impl Default for LaunchMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub mode: LaunchMode,
    pub sandbox: bool,
    pub focus_on_launch: bool,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            mode: LaunchMode::Normal,
            sandbox: false,
            focus_on_launch: true,
        }
    }
}

pub struct AppLauncher {
    pub apps: HashMap<String, AppEntry>,
    pub running_apps: HashMap<String, u32>,
    pub launch_counts: HashMap<String, u32>,
    pub config: LaunchConfig,
}

impl AppLauncher {
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
            running_apps: HashMap::new(),
            launch_counts: HashMap::new(),
            config: LaunchConfig::default(),
        }
    }

    pub fn register_app(&mut self, app: AppEntry) {
        self.apps.insert(app.id.clone(), app);
    }

    pub fn unregister_app(&mut self, id: &str) {
        self.apps.remove(id);
    }

    pub fn get_app(&self, id: &str) -> Option<&AppEntry> {
        self.apps.get(id)
    }

    pub fn launch(&mut self, id: &str) -> anyhow::Result<u32> {
        let app = self.apps.get(id).ok_or_else(|| anyhow::anyhow!("App not found: {}", id))?;
        let pid = app.launch()?;
        self.running_apps.insert(id.to_string(), pid);
        *self.launch_counts.entry(id.to_string()).or_insert(0) += 1;
        Ok(pid)
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running_apps.contains_key(id)
    }
}

impl Default for AppLauncher {
    fn default() -> Self {
        Self::new()
    }
}