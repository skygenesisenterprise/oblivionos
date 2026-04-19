use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputHandle(u32);

impl OutputHandle {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub handle: OutputHandle,
    pub name: String,
    pub make: String,
    pub model: String,
    pub physical_size: (u32, u32),
    pub position: (i32, i32),
    pub resolution: (u32, u32),
    pub refresh_rate: f64,
    pub scale: f64,
    pub enabled: bool,
    pub primary: bool,
    pub hdr: bool,
    pub color_space: ColorSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Srgb,
    DisplayP3,
    Rec2020,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self {
            handle: OutputHandle::new(0),
            name: "DP-1".to_string(),
            make: "Generic".to_string(),
            model: "Display".to_string(),
            physical_size: (600, 340),
            position: (0, 0),
            resolution: (1920, 1080),
            refresh_rate: 60.0,
            scale: 1.0,
            enabled: true,
            primary: true,
            hdr: false,
            color_space: ColorSpace::Srgb,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct OutputId(String);

impl OutputId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
}

pub struct OutputManager {
    outputs: HashMap<OutputId, OutputInfo>,
    primary_output: Option<OutputId>,
    next_id: u32,
}

impl OutputManager {
    pub fn new() -> Self {
        let mut manager = Self {
            outputs: HashMap::new(),
            primary_output: None,
            next_id: 1,
        };

        manager.add_output(OutputInfo::default());
        manager
    }

    pub fn add_output(&mut self, mut info: OutputInfo) -> OutputId {
        let id = OutputId::new(&format!("{}-{}", info.make, self.next_id));
        self.next_id += 1;

        info.handle = OutputHandle::new(self.next_id);

        if info.primary || self.primary_output.is_none() {
            self.primary_output = Some(id.clone());
        }

        self.outputs.insert(id.clone(), info);
        tracing::info!("Added output: {:?}", id);
        id
    }

    pub fn remove_output(&mut self, id: &OutputId) {
        if self.primary_output.as_ref() == Some(id) {
            self.primary_output = self.outputs.keys().next().cloned();
        }
        self.outputs.remove(id);
    }

    pub fn get_output(&self, id: &OutputId) -> Option<&OutputInfo> {
        self.outputs.get(id)
    }

    pub fn get_all_outputs(&self) -> Vec<&OutputInfo> {
        self.outputs.values().collect()
    }

    pub fn get_primary(&self) -> Option<&OutputInfo> {
        self.primary_output
            .as_ref()
            .and_then(|id| self.outputs.get(id))
    }
}

impl Default for OutputManager {
    fn default() -> Self {
        Self::new()
    }
}