use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct KillRing {
    entries: VecDeque<String>,
    capacity: usize,
    yank_pointer: usize,
    last_was_kill: bool,
}

impl Default for KillRing {
    fn default() -> Self {
        Self::new(60)
    }
}

impl KillRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            yank_pointer: 0,
            last_was_kill: false,
        }
    }

    pub fn push(&mut self, text: String, append: bool) {
        if text.is_empty() {
            return;
        }

        if append && self.last_was_kill && !self.entries.is_empty() {
            let current = self.entries.front_mut().unwrap();
            current.push_str(&text);
        } else {
            if self.entries.len() >= self.capacity {
                self.entries.pop_back();
            }
            self.entries.push_front(text);
        }

        self.yank_pointer = 0;
        self.last_was_kill = true;
    }

    pub fn push_prepend(&mut self, text: String) {
        if text.is_empty() {
            return;
        }

        if self.last_was_kill && !self.entries.is_empty() {
            let current = self.entries.front_mut().unwrap();
            let old = std::mem::take(current);
            *current = text + &old;
        } else {
            if self.entries.len() >= self.capacity {
                self.entries.pop_back();
            }
            self.entries.push_front(text);
        }

        self.yank_pointer = 0;
        self.last_was_kill = true;
    }

    pub fn current(&self) -> Option<&str> {
        self.entries.get(self.yank_pointer).map(|s| s.as_str())
    }

    pub fn yank(&self) -> Option<&str> {
        self.entries.front().map(|s| s.as_str())
    }

    pub fn yank_pop(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        self.yank_pointer = (self.yank_pointer + 1) % self.entries.len();
        self.entries.get(self.yank_pointer).map(|s| s.as_str())
    }

    pub fn reset_yank_pointer(&mut self) {
        self.yank_pointer = 0;
    }

    pub fn set_last_was_kill(&mut self, was_kill: bool) {
        self.last_was_kill = was_kill;
    }

    pub fn last_was_kill(&self) -> bool {
        self.last_was_kill
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_ring_basic() {
        let mut ring = KillRing::new(3);
        ring.push("first".into(), false);
        ring.set_last_was_kill(false);
        ring.push("second".into(), false);
        ring.set_last_was_kill(false);
        ring.push("third".into(), false);

        assert_eq!(ring.yank(), Some("third"));
        assert_eq!(ring.yank_pop(), Some("second"));
        assert_eq!(ring.yank_pop(), Some("first"));
        assert_eq!(ring.yank_pop(), Some("third"));
    }

    #[test]
    fn test_kill_ring_append() {
        let mut ring = KillRing::new(10);
        ring.push("hello".into(), false);
        ring.push(" world".into(), true);

        assert_eq!(ring.yank(), Some("hello world"));
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_kill_ring_prepend() {
        let mut ring = KillRing::new(10);
        ring.push("world".into(), false);
        ring.push_prepend("hello ".into());

        assert_eq!(ring.yank(), Some("hello world"));
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_kill_ring_capacity() {
        let mut ring = KillRing::new(2);
        ring.push("first".into(), false);
        ring.set_last_was_kill(false);
        ring.push("second".into(), false);
        ring.set_last_was_kill(false);
        ring.push("third".into(), false);

        assert_eq!(ring.len(), 2);
        let entries: Vec<_> = ring.iter().collect();
        assert_eq!(entries, vec!["third", "second"]);
    }
}
