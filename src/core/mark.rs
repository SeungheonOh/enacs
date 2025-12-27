use super::position::CharOffset;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark(pub CharOffset);

impl Mark {
    pub fn new(position: CharOffset) -> Self {
        Self(position)
    }

    pub fn position(&self) -> CharOffset {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct MarkRing {
    marks: VecDeque<Mark>,
    capacity: usize,
}

impl Default for MarkRing {
    fn default() -> Self {
        Self::new(16)
    }
}

impl MarkRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            marks: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, mark: Mark) {
        if self.marks.front() == Some(&mark) {
            return;
        }

        if self.marks.len() >= self.capacity {
            self.marks.pop_back();
        }
        self.marks.push_front(mark);
    }

    pub fn pop(&mut self) -> Option<Mark> {
        self.marks.pop_front()
    }

    pub fn current(&self) -> Option<&Mark> {
        self.marks.front()
    }

    pub fn rotate(&mut self) {
        if let Some(mark) = self.marks.pop_front() {
            self.marks.push_back(mark);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn clear(&mut self) {
        self.marks.clear();
    }

    pub fn adjust_after_insert(&mut self, insert_pos: CharOffset, len: usize) {
        for mark in self.marks.iter_mut() {
            if mark.0 >= insert_pos {
                mark.0 = CharOffset(mark.0 .0 + len);
            }
        }
    }

    pub fn adjust_after_delete(&mut self, delete_start: CharOffset, delete_end: CharOffset) {
        let deleted_len = delete_end.0 - delete_start.0;
        for mark in self.marks.iter_mut() {
            if mark.0 >= delete_end {
                mark.0 = CharOffset(mark.0 .0 - deleted_len);
            } else if mark.0 > delete_start {
                mark.0 = delete_start;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_ring_push_and_pop() {
        let mut ring = MarkRing::new(3);
        ring.push(Mark::new(CharOffset(10)));
        ring.push(Mark::new(CharOffset(20)));
        ring.push(Mark::new(CharOffset(30)));

        assert_eq!(ring.len(), 3);
        assert_eq!(ring.pop().map(|m| m.0), Some(CharOffset(30)));
        assert_eq!(ring.pop().map(|m| m.0), Some(CharOffset(20)));
        assert_eq!(ring.pop().map(|m| m.0), Some(CharOffset(10)));
    }

    #[test]
    fn test_mark_ring_capacity() {
        let mut ring = MarkRing::new(2);
        ring.push(Mark::new(CharOffset(10)));
        ring.push(Mark::new(CharOffset(20)));
        ring.push(Mark::new(CharOffset(30)));

        assert_eq!(ring.len(), 2);
        assert_eq!(ring.pop().map(|m| m.0), Some(CharOffset(30)));
        assert_eq!(ring.pop().map(|m| m.0), Some(CharOffset(20)));
    }

    #[test]
    fn test_mark_ring_rotate() {
        let mut ring = MarkRing::new(3);
        ring.push(Mark::new(CharOffset(10)));
        ring.push(Mark::new(CharOffset(20)));
        ring.push(Mark::new(CharOffset(30)));

        ring.rotate();
        assert_eq!(ring.current().map(|m| m.0), Some(CharOffset(20)));

        ring.rotate();
        assert_eq!(ring.current().map(|m| m.0), Some(CharOffset(10)));
    }
}
