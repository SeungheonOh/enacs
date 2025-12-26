use super::cursor::CursorSet;
use super::position::CharOffset;

#[derive(Debug, Clone)]
pub enum Edit {
    Insert {
        position: CharOffset,
        text: String,
    },
    Delete {
        position: CharOffset,
        text: String,
    },
}

impl Edit {
    fn inverse(&self) -> Edit {
        match self {
            Edit::Insert { position, text } => Edit::Delete {
                position: *position,
                text: text.clone(),
            },
            Edit::Delete { position, text } => Edit::Insert {
                position: *position,
                text: text.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub edits: Vec<Edit>,
    pub cursors_before: Option<CursorSet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceMode {
    Insertion,
    Deletion,
    None,
}

#[derive(Debug)]
pub struct UndoTree {
    entries: Vec<UndoEntry>,
    undo_index: Option<usize>,
    pending_edits: Vec<Edit>,
    pending_cursors: Option<CursorSet>,
    coalesce_mode: CoalesceMode,
    last_insert_end: Option<CharOffset>,
    last_delete_pos: Option<CharOffset>,
    max_entries: usize,
}

impl Default for UndoTree {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl UndoTree {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            undo_index: None,
            pending_edits: Vec::new(),
            pending_cursors: None,
            coalesce_mode: CoalesceMode::None,
            last_insert_end: None,
            last_delete_pos: None,
            max_entries,
        }
    }

    fn is_word_boundary(c: char) -> bool {
        c.is_whitespace() || c.is_ascii_punctuation()
    }

    fn should_coalesce_insert(&self, position: CharOffset, text: &str) -> bool {
        if self.coalesce_mode != CoalesceMode::Insertion {
            return false;
        }

        if let Some(last_end) = self.last_insert_end {
            if position != last_end {
                return false;
            }
        } else {
            return false;
        }

        if text.len() != 1 {
            return false;
        }

        let c = text.chars().next().unwrap();
        if Self::is_word_boundary(c) {
            return false;
        }

        if let Some(Edit::Insert { text: pending_text, .. }) = self.pending_edits.last() {
            if let Some(last_char) = pending_text.chars().last() {
                if Self::is_word_boundary(last_char) {
                    return false;
                }
            }
        }

        true
    }

    fn should_coalesce_delete(&self, position: CharOffset, text: &str) -> bool {
        if self.coalesce_mode != CoalesceMode::Deletion {
            return false;
        }

        let text_len = text.chars().count();
        if text_len != 1 {
            return false;
        }

        if let Some(last_pos) = self.last_delete_pos {
            let is_backspace = CharOffset(position.0 + text_len) == last_pos;
            let is_forward_delete = position == last_pos;
            if !is_backspace && !is_forward_delete {
                return false;
            }
        } else {
            return false;
        }

        let c = text.chars().next().unwrap();
        !Self::is_word_boundary(c)
    }

    pub fn record_insert(&mut self, position: CharOffset, text: String) {
        self.undo_index = None;
        let text_len = text.chars().count();

        if self.should_coalesce_insert(position, &text) {
            if let Some(Edit::Insert { text: ref mut existing, .. }) = self.pending_edits.last_mut() {
                existing.push_str(&text);
                self.last_insert_end = Some(CharOffset(position.0 + text_len));
                return;
            }
        }

        self.flush_pending();

        self.pending_edits.push(Edit::Insert {
            position,
            text: text.clone(),
        });
        self.coalesce_mode = CoalesceMode::Insertion;
        self.last_insert_end = Some(CharOffset(position.0 + text_len));
        self.last_delete_pos = None;
    }

    pub fn record_delete(&mut self, position: CharOffset, text: String) {
        self.undo_index = None;

        if self.should_coalesce_delete(position, &text) {
            if let Some(Edit::Delete { position: ref mut del_pos, text: ref mut existing }) = self.pending_edits.last_mut() {
                let is_backspace = position.0 < del_pos.0;
                if is_backspace {
                    existing.insert_str(0, &text);
                    *del_pos = position;
                } else {
                    existing.push_str(&text);
                }
                self.last_delete_pos = Some(position);
                return;
            }
        }

        self.flush_pending();

        self.pending_edits.push(Edit::Delete {
            position,
            text: text.clone(),
        });
        self.coalesce_mode = CoalesceMode::Deletion;
        self.last_delete_pos = Some(position);
        self.last_insert_end = None;
    }

    pub fn set_cursors_before(&mut self, cursors: CursorSet) {
        if self.pending_cursors.is_none() {
            self.pending_cursors = Some(cursors);
        }
    }

    pub fn break_coalesce(&mut self) {
        self.flush_pending();
        self.coalesce_mode = CoalesceMode::None;
        self.last_insert_end = None;
        self.last_delete_pos = None;
    }

    fn flush_pending(&mut self) {
        if self.pending_edits.is_empty() {
            return;
        }

        let entry = UndoEntry {
            edits: std::mem::take(&mut self.pending_edits),
            cursors_before: self.pending_cursors.take(),
        };
        self.entries.push(entry);

        self.coalesce_mode = CoalesceMode::None;

        self.gc_if_needed();
    }

    pub fn add_boundary(&mut self) {
        self.break_coalesce();
    }

    pub fn undo(&mut self) -> UndoResult {
        self.flush_pending();

        let idx = self.undo_index.unwrap_or(self.entries.len());
        if idx == 0 {
            return UndoResult::Nothing;
        }

        let entry = &self.entries[idx - 1];
        let cursors = entry.cursors_before.clone();

        let inverse_edits: Vec<Edit> = entry.edits.iter().rev().map(|e| e.inverse()).collect();

        self.entries.push(UndoEntry {
            edits: inverse_edits.clone(),
            cursors_before: None,
        });

        self.undo_index = Some(idx - 1);

        UndoResult::Apply {
            edits: inverse_edits
                .into_iter()
                .map(|e| match e {
                    Edit::Insert { position, text } => UndoEdit::Insert { position, text },
                    Edit::Delete { position, text } => UndoEdit::Delete {
                        position,
                        len: text.chars().count(),
                    },
                })
                .collect(),
            restore_cursors: cursors,
        }
    }

    pub fn redo(&mut self) -> UndoResult {
        self.undo()
    }

    pub fn can_undo(&self) -> bool {
        let idx = self.undo_index.unwrap_or(self.entries.len());
        idx > 0 || !self.pending_edits.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        self.can_undo()
    }

    fn gc_if_needed(&mut self) {
        if self.entries.len() <= self.max_entries {
            return;
        }
        let remove_count = self.entries.len() - self.max_entries;
        self.entries.drain(0..remove_count);
        if let Some(idx) = self.undo_index {
            self.undo_index = Some(idx.saturating_sub(remove_count));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.undo_index = None;
        self.pending_edits.clear();
        self.pending_cursors = None;
        self.coalesce_mode = CoalesceMode::None;
        self.last_insert_end = None;
        self.last_delete_pos = None;
    }
}

#[derive(Debug, Clone)]
pub enum UndoEdit {
    Insert { position: CharOffset, text: String },
    Delete { position: CharOffset, len: usize },
}

#[derive(Debug)]
pub enum UndoResult {
    Apply {
        edits: Vec<UndoEdit>,
        restore_cursors: Option<CursorSet>,
    },
    Nothing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_insert() {
        let mut tree = UndoTree::new(100);
        tree.record_insert(CharOffset(0), "hello".into());
        tree.add_boundary();

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { position, len } => {
                        assert_eq!(*position, CharOffset(0));
                        assert_eq!(*len, 5);
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_undo_delete() {
        let mut tree = UndoTree::new(100);
        tree.record_delete(CharOffset(0), "hello".into());
        tree.add_boundary();

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Insert { position, text } => {
                        assert_eq!(*position, CharOffset(0));
                        assert_eq!(text, "hello");
                    }
                    _ => panic!("Expected insert"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_coalesce_insert() {
        let mut tree = UndoTree::new(100);
        tree.record_insert(CharOffset(0), "h".into());
        tree.record_insert(CharOffset(1), "e".into());
        tree.record_insert(CharOffset(2), "l".into());
        tree.record_insert(CharOffset(3), "l".into());
        tree.record_insert(CharOffset(4), "o".into());
        tree.add_boundary();

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { position, len } => {
                        assert_eq!(*position, CharOffset(0));
                        assert_eq!(*len, 5);
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_coalesce_breaks_on_space() {
        let mut tree = UndoTree::new(100);
        tree.record_insert(CharOffset(0), "h".into());
        tree.record_insert(CharOffset(1), "i".into());
        tree.record_insert(CharOffset(2), " ".into());
        tree.record_insert(CharOffset(3), "y".into());
        tree.record_insert(CharOffset(4), "o".into());
        tree.add_boundary();

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 2);
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 1);
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 2);
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_emacs_style_undo_traversal() {
        let mut tree = UndoTree::new(100);

        tree.record_insert(CharOffset(0), "foo\n".into());
        tree.add_boundary();
        tree.record_insert(CharOffset(4), "bar\n".into());
        tree.add_boundary();
        tree.record_insert(CharOffset(8), "baz\n".into());
        tree.add_boundary();
        tree.record_insert(CharOffset(12), "faz".into());
        tree.add_boundary();

        tree.undo();
        tree.undo();

        tree.record_insert(CharOffset(8), "hello\n".into());
        tree.add_boundary();
        tree.record_insert(CharOffset(14), "world".into());
        tree.add_boundary();

        tree.undo();
        tree.undo();

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                match &edits[0] {
                    UndoEdit::Insert { text, .. } => {
                        assert_eq!(text, "baz\n");
                    }
                    _ => panic!("Expected insert"),
                }
            }
            _ => panic!("Expected Apply"),
        }

        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                match &edits[0] {
                    UndoEdit::Insert { text, .. } => {
                        assert_eq!(text, "faz");
                    }
                    _ => panic!("Expected insert"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }
}
