mod dock;
mod menubar;
mod decorations;
mod appfinder;
mod expose;
mod launcher;
mod workspace;
mod tray;

pub struct Shell {
    pub dock: dock::Dock,
    pub menu_bar: menubar::MenuBar,
    pub workspace_manager: workspace::WorkspaceManager,
    pub appfinder: appfinder::AppFinder,
    pub app_launcher: launcher::AppLauncher,
    pub expose_manager: expose::ExposeManager,
    pub system_tray: tray::SystemTray,
}

impl Shell {
    pub fn new() -> anyhow::Result<Self> {
        let dock = dock::Dock::new()?;
        let menu_bar = menubar::MenuBar::new()?;
        let workspace_manager = workspace::WorkspaceManager::new();
        let appfinder = appfinder::AppFinder::new();
        let app_launcher = launcher::AppLauncher::new();
        let expose_manager = expose::ExposeManager::new();
        let system_tray = tray::SystemTray::new();

        tracing::info!("Shell initialized");

        Ok(Self {
            dock,
            menu_bar,
            workspace_manager,
            appfinder,
            app_launcher,
            expose_manager,
            system_tray,
        })
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new().expect("Failed to create shell")
    }
}