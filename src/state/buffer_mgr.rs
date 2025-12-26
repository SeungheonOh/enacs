use std::collections::HashMap;

use crate::core::{Buffer, BufferId};

#[derive(Debug, Default)]
pub struct BufferManager {
    buffers: HashMap<BufferId, Buffer>,
    order: Vec<BufferId>,
    current: Option<BufferId>,
}

impl BufferManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, buffer: Buffer) -> BufferId {
        let id = buffer.id;
        self.buffers.insert(id, buffer);
        self.order.push(id);
        if self.current.is_none() {
            self.current = Some(id);
        }
        id
    }

    pub fn get(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.get(&id)
    }

    pub fn get_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(&id)
    }

    pub fn current_id(&self) -> Option<BufferId> {
        self.current
    }

    pub fn current(&self) -> Option<&Buffer> {
        self.current.and_then(|id| self.buffers.get(&id))
    }

    pub fn current_mut(&mut self) -> Option<&mut Buffer> {
        self.current.and_then(|id| self.buffers.get_mut(&id))
    }

    pub fn set_current(&mut self, id: BufferId) -> bool {
        if self.buffers.contains_key(&id) {
            self.current = Some(id);
            if let Some(pos) = self.order.iter().position(|&i| i == id) {
                self.order.remove(pos);
                self.order.insert(0, id);
            }
            true
        } else {
            false
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<BufferId> {
        self.buffers
            .iter()
            .find(|(_, buf)| buf.name == name)
            .map(|(id, _)| *id)
    }

    pub fn switch_to_name(&mut self, name: &str) -> bool {
        if let Some(id) = self.find_by_name(name) {
            self.set_current(id)
        } else {
            false
        }
    }

    pub fn kill(&mut self, id: BufferId) -> Option<Buffer> {
        if let Some(buffer) = self.buffers.remove(&id) {
            self.order.retain(|&i| i != id);
            if self.current == Some(id) {
                self.current = self.order.first().copied();
            }
            Some(buffer)
        } else {
            None
        }
    }

    pub fn kill_current(&mut self) -> Option<Buffer> {
        self.current.and_then(|id| self.kill(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Buffer> {
        self.order.iter().filter_map(|id| self.buffers.get(id))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Buffer> {
        self.buffers.values_mut()
    }

    pub fn count(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    pub fn names(&self) -> Vec<&str> {
        self.order
            .iter()
            .filter_map(|id| self.buffers.get(id).map(|b| b.name.as_str()))
            .collect()
    }

    pub fn ensure_scratch(&mut self) -> BufferId {
        if let Some(id) = self.find_by_name("*scratch*") {
            return id;
        }

        let scratch = Buffer::new("*scratch*");
        self.add(scratch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_manager_add_and_get() {
        let mut mgr = BufferManager::new();
        let buffer = Buffer::new("test");
        let id = mgr.add(buffer);

        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.get(id).unwrap().name, "test");
    }

    #[test]
    fn test_buffer_manager_current() {
        let mut mgr = BufferManager::new();
        let buffer1 = Buffer::new("buf1");
        let buffer2 = Buffer::new("buf2");

        let id1 = mgr.add(buffer1);
        let id2 = mgr.add(buffer2);

        assert_eq!(mgr.current_id(), Some(id1));

        mgr.set_current(id2);
        assert_eq!(mgr.current_id(), Some(id2));
    }

    #[test]
    fn test_buffer_manager_kill() {
        let mut mgr = BufferManager::new();
        let buffer1 = Buffer::new("buf1");
        let buffer2 = Buffer::new("buf2");

        let id1 = mgr.add(buffer1);
        let id2 = mgr.add(buffer2);

        mgr.set_current(id1);
        let killed = mgr.kill(id1);

        assert!(killed.is_some());
        assert!(mgr.get(id1).is_none());
        assert_eq!(mgr.current_id(), Some(id2));
    }
}
