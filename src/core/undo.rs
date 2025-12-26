use super::cursor::CursorSet;
use super::position::CharOffset;

#[derive(Debug, Clone)]
pub enum UndoEntry {
    Insert {
        position: CharOffset,
        text: String,
    },
    Delete {
        position: CharOffset,
        text: String,
    },
    CursorState {
        cursors: CursorSet,
    },
    Boundary,
}

#[derive(Debug, Clone)]
pub struct UndoHistory {
    entries: Vec<UndoEntry>,
    position: usize,
    in_undo_sequence: bool,
    max_entries: usize,
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl UndoHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            position: 0,
            in_undo_sequence: false,
            max_entries,
        }
    }

    pub fn record_insert(&mut self, position: CharOffset, text: String) {
        if self.in_undo_sequence {
            self.in_undo_sequence = false;
            self.push(UndoEntry::Boundary);
        }
        self.push(UndoEntry::Insert { position, text });
    }

    pub fn record_delete(&mut self, position: CharOffset, text: String) {
        if self.in_undo_sequence {
            self.in_undo_sequence = false;
            self.push(UndoEntry::Boundary);
        }
        self.push(UndoEntry::Delete { position, text });
    }

    pub fn record_cursor_state(&mut self, cursors: CursorSet) {
        self.push(UndoEntry::CursorState { cursors });
    }

    pub fn add_boundary(&mut self) {
        if !matches!(self.entries.last(), Some(UndoEntry::Boundary) | None) {
            self.push(UndoEntry::Boundary);
        }
    }

    fn push(&mut self, entry: UndoEntry) {
        self.entries.push(entry);
        self.position = self.entries.len();

        if self.entries.len() > self.max_entries {
            let remove_count = self.entries.len() - self.max_entries;
            self.entries.drain(0..remove_count);
            self.position = self.entries.len();
        }
    }

    pub fn start_undo_sequence(&mut self) {
        self.in_undo_sequence = true;
    }

    pub fn pop_undo(&mut self) -> Option<UndoEntry> {
        if self.position == 0 {
            return None;
        }

        self.position -= 1;
        let entry = self.entries.get(self.position).cloned();

        if matches!(entry, Some(UndoEntry::Boundary)) {
            return self.pop_undo();
        }

        entry
    }

    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.position = 0;
        self.in_undo_sequence = false;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug)]
pub enum UndoResult {
    Insert { position: CharOffset, text: String },
    Delete { position: CharOffset, len: usize },
    RestoreCursors(CursorSet),
    Nothing,
}

impl UndoHistory {
    pub fn undo_one(&mut self) -> UndoResult {
        match self.pop_undo() {
            Some(UndoEntry::Insert { position, text }) => {
                UndoResult::Delete {
                    position,
                    len: text.chars().count(),
                }
            }
            Some(UndoEntry::Delete { position, text }) => {
                UndoResult::Insert { position, text }
            }
            Some(UndoEntry::CursorState { cursors }) => {
                UndoResult::RestoreCursors(cursors)
            }
            Some(UndoEntry::Boundary) | None => UndoResult::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_insert() {
        let mut history = UndoHistory::new(100);
        history.record_insert(CharOffset(0), "hello".into());

        match history.undo_one() {
            UndoResult::Delete { position, len } => {
                assert_eq!(position, CharOffset(0));
                assert_eq!(len, 5);
            }
            _ => panic!("Expected delete"),
        }
    }

    #[test]
    fn test_undo_delete() {
        let mut history = UndoHistory::new(100);
        history.record_delete(CharOffset(0), "hello".into());

        match history.undo_one() {
            UndoResult::Insert { position, text } => {
                assert_eq!(position, CharOffset(0));
                assert_eq!(text, "hello");
            }
            _ => panic!("Expected insert"),
        }
    }

    #[test]
    fn test_boundary() {
        let mut history = UndoHistory::new(100);
        history.record_insert(CharOffset(0), "a".into());
        history.add_boundary();
        history.record_insert(CharOffset(1), "b".into());

        match history.undo_one() {
            UndoResult::Delete { position, .. } => {
                assert_eq!(position, CharOffset(1));
            }
            _ => panic!("Expected delete"),
        }

        match history.undo_one() {
            UndoResult::Delete { position, .. } => {
                assert_eq!(position, CharOffset(0));
            }
            _ => panic!("Expected delete"),
        }
    }
}
