use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::cursor::CursorSet;
use crate::core::BufferId;

static WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

impl WindowId {
    pub fn new() -> Self {
        Self(WINDOW_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub buffer_id: BufferId,
    pub cursors: CursorSet,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub scroll_line: usize,
    pub scroll_column: usize,
}

impl Window {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            id: WindowId::new(),
            buffer_id,
            cursors: CursorSet::new(),
            x: 0,
            y: 0,
            width: 80,
            height: 24,
            scroll_line: 0,
            scroll_column: 0,
        }
    }

    pub fn with_dimensions(buffer_id: BufferId, x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            id: WindowId::new(),
            buffer_id,
            cursors: CursorSet::new(),
            x,
            y,
            width,
            height,
            scroll_line: 0,
            scroll_column: 0,
        }
    }
}

#[derive(Debug)]
pub struct WindowManager {
    windows: Vec<Window>,
    current: usize,
    total_width: u16,
    total_height: u16,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            current: 0,
            total_width: 80,
            total_height: 24,
        }
    }
}

impl WindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dimensions(width: u16, height: u16) -> Self {
        Self {
            windows: Vec::new(),
            current: 0,
            total_width: width,
            total_height: height,
        }
    }

    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.total_width = width;
        self.total_height = height;
        self.relayout();
    }

    pub fn add(&mut self, buffer_id: BufferId) -> WindowId {
        let window = Window::with_dimensions(
            buffer_id,
            0,
            0,
            self.total_width,
            self.total_height.saturating_sub(1),
        );
        let id = window.id;
        self.windows.push(window);
        self.relayout();
        id
    }

    pub fn current(&self) -> Option<&Window> {
        self.windows.get(self.current)
    }

    pub fn current_mut(&mut self) -> Option<&mut Window> {
        self.windows.get_mut(self.current)
    }

    pub fn current_buffer_id(&self) -> Option<BufferId> {
        self.current().map(|w| w.buffer_id)
    }

    pub fn set_current_buffer(&mut self, buffer_id: BufferId) {
        if let Some(window) = self.current_mut() {
            window.buffer_id = buffer_id;
            window.scroll_line = 0;
            window.scroll_column = 0;
        }
    }

    pub fn cycle_next(&mut self) {
        if !self.windows.is_empty() {
            self.current = (self.current + 1) % self.windows.len();
        }
    }

    pub fn cycle_prev(&mut self) {
        if !self.windows.is_empty() {
            self.current = if self.current == 0 {
                self.windows.len() - 1
            } else {
                self.current - 1
            };
        }
    }

    pub fn split_vertical(&mut self) {
        if let Some(current) = self.windows.get(self.current).cloned() {
            let half_height = current.height / 2;
            let new_window = Window::with_dimensions(
                current.buffer_id,
                current.x,
                current.y + half_height,
                current.width,
                current.height - half_height,
            );

            if let Some(window) = self.windows.get_mut(self.current) {
                window.height = half_height;
            }

            self.windows.insert(self.current + 1, new_window);
            self.relayout();
        }
    }

    pub fn split_horizontal(&mut self) {
        if let Some(current) = self.windows.get(self.current).cloned() {
            let half_width = current.width / 2;
            let new_window = Window::with_dimensions(
                current.buffer_id,
                current.x + half_width,
                current.y,
                current.width - half_width,
                current.height,
            );

            if let Some(window) = self.windows.get_mut(self.current) {
                window.width = half_width;
            }

            self.windows.insert(self.current + 1, new_window);
        }
    }

    pub fn delete_current(&mut self) {
        if self.windows.len() > 1 {
            self.windows.remove(self.current);
            if self.current >= self.windows.len() {
                self.current = self.windows.len() - 1;
            }
            self.relayout();
        }
    }

    pub fn delete_others(&mut self) {
        if let Some(current) = self.windows.get(self.current).cloned() {
            self.windows.clear();
            self.windows.push(current);
            self.current = 0;
            self.relayout();
        }
    }

    pub fn count(&self) -> usize {
        self.windows.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Window> {
        self.windows.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.windows.iter_mut()
    }

    fn relayout(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        let usable_height = self.total_height.saturating_sub(1);
        let n = self.windows.len() as u16;
        let base_height = usable_height / n;
        let remainder = usable_height % n;

        let mut y = 0u16;
        for (i, window) in self.windows.iter_mut().enumerate() {
            let extra = if (i as u16) < remainder { 1 } else { 0 };
            window.x = 0;
            window.y = y;
            window.width = self.total_width;
            window.height = base_height + extra;
            y += window.height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_manager_add() {
        let mut mgr = WindowManager::with_dimensions(80, 24);
        let buffer_id = BufferId(1);
        mgr.add(buffer_id);

        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.current_buffer_id(), Some(buffer_id));
    }

    #[test]
    fn test_window_split_vertical() {
        let mut mgr = WindowManager::with_dimensions(80, 24);
        let buffer_id = BufferId(1);
        mgr.add(buffer_id);
        mgr.split_vertical();

        assert_eq!(mgr.count(), 2);
    }

    #[test]
    fn test_window_cycle() {
        let mut mgr = WindowManager::with_dimensions(80, 24);
        mgr.add(BufferId(1));
        mgr.add(BufferId(2));

        assert_eq!(mgr.current, 0);
        mgr.cycle_next();
        assert_eq!(mgr.current, 1);
        mgr.cycle_next();
        assert_eq!(mgr.current, 0);
    }
}
