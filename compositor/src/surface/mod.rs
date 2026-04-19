use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(u32);

impl SurfaceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceRole {
    Toplevel,
    Popup,
    Subsurface,
    Layer,
    Cursor,
}

#[derive(Debug, Clone)]
pub struct OblivionSurface {
    pub id: SurfaceId,
    pub role: SurfaceRole,
    pub parent: Option<SurfaceId>,
    pub children: Vec<SurfaceId>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl OblivionSurface {
    pub fn new(id: SurfaceId, role: SurfaceRole) -> Self {
        Self {
            id,
            role,
            parent: None,
            children: Vec::new(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceTree {
    surfaces: Vec<OblivionSurface>,
    next_id: u32,
}

impl SurfaceTree {
    pub fn new() -> Self {
        Self {
            surfaces: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_surface(&mut self, role: SurfaceRole) -> SurfaceId {
        let id = SurfaceId(self.next_id);
        self.next_id += 1;
        let surface = OblivionSurface::new(id, role);
        self.surfaces.push(surface);
        id
    }

    pub fn get_surface(&self, id: SurfaceId) -> Option<&OblivionSurface> {
        self.surfaces.iter().find(|s| s.id == id)
    }

    pub fn get_toplevels(&self) -> Vec<&OblivionSurface> {
        self.surfaces.iter().filter(|s| matches!(s.role, SurfaceRole::Toplevel)).collect()
    }
}

impl Default for SurfaceTree {
    fn default() -> Self {
        Self::new()
    }
}