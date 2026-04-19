use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(u64);

impl BufferId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct Buffer {
    pub id: BufferId,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub stride: u32,
    pub ready: bool,
}

pub struct BufferManager {
    buffers: VecDeque<Buffer>,
    next_id: u64,
    max_buffers: usize,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: VecDeque::new(),
            next_id: 1,
            max_buffers: 256,
        }
    }

    pub fn create_buffer(&mut self, width: u32, height: u32, format: u32, stride: u32) -> BufferId {
        let id = BufferId(self.next_id);
        self.next_id += 1;
        
        if self.buffers.len() >= self.max_buffers {
            self.buffers.pop_front();
        }

        let buffer = Buffer {
            id,
            width,
            height,
            format,
            stride,
            ready: false,
        };

        self.buffers.push_back(buffer);
        id
    }

    pub fn get_buffer(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.iter().find(|b| b.id == id)
    }

    pub fn mark_ready(&mut self, id: BufferId) {
        if let Some(buffer) = self.buffers.iter_mut().find(|b| b.id == id) {
            buffer.ready = true;
        }
    }

    pub fn release_buffer(&mut self, id: BufferId) {
        self.buffers.retain(|b| b.id != id);
    }

    pub fn cleanup(&mut self) {
        let ready_ids: Vec<_> = self.buffers
            .iter()
            .filter(|b| b.ready)
            .map(|b| b.id)
            .collect();

        for id in ready_ids {
            self.release_buffer(id);
        }
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}