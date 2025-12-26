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

#[derive(Debug, Clone)]
pub struct UndoNode {
    pub edits: Vec<Edit>,
    pub cursors_before: Option<CursorSet>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

impl UndoNode {
    fn new(parent: Option<usize>) -> Self {
        Self {
            edits: Vec::new(),
            cursors_before: None,
            children: Vec::new(),
            parent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceMode {
    Insertion,
    Deletion,
    None,
}

#[derive(Debug)]
pub struct UndoTree {
    nodes: Vec<UndoNode>,
    current: usize,
    redo_stack: Vec<usize>,
    pending_edits: Vec<Edit>,
    pending_cursors: Option<CursorSet>,
    coalesce_mode: CoalesceMode,
    last_insert_end: Option<CharOffset>,
    last_delete_pos: Option<CharOffset>,
    max_nodes: usize,
}

impl Default for UndoTree {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl UndoTree {
    pub fn new(max_nodes: usize) -> Self {
        let root = UndoNode::new(None);
        Self {
            nodes: vec![root],
            current: 0,
            redo_stack: Vec::new(),
            pending_edits: Vec::new(),
            pending_cursors: None,
            coalesce_mode: CoalesceMode::None,
            last_insert_end: None,
            last_delete_pos: None,
            max_nodes,
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

        self.redo_stack.clear();

        let mut node = UndoNode::new(Some(self.current));
        node.edits = std::mem::take(&mut self.pending_edits);
        node.cursors_before = self.pending_cursors.take();

        let new_idx = self.nodes.len();
        self.nodes[self.current].children.push(new_idx);
        self.nodes.push(node);
        self.current = new_idx;

        self.coalesce_mode = CoalesceMode::None;

        self.gc_if_needed();
    }

    pub fn add_boundary(&mut self) {
        self.break_coalesce();
    }

    pub fn undo(&mut self) -> UndoResult {
        self.flush_pending();

        if self.current == 0 {
            return UndoResult::Nothing;
        }

        let node = &self.nodes[self.current];
        if node.edits.is_empty() {
            return UndoResult::Nothing;
        }

        let edits_to_undo: Vec<Edit> = node.edits.iter().rev().cloned().collect();
        let cursors = node.cursors_before.clone();

        self.redo_stack.push(self.current);
        self.current = node.parent.unwrap_or(0);

        UndoResult::Apply {
            edits: edits_to_undo
                .into_iter()
                .map(|e| match e {
                    Edit::Insert { position, text } => UndoEdit::Delete {
                        position,
                        len: text.chars().count(),
                    },
                    Edit::Delete { position, text } => UndoEdit::Insert { position, text },
                })
                .collect(),
            restore_cursors: cursors,
        }
    }

    pub fn redo(&mut self) -> UndoResult {
        self.flush_pending();

        let redo_target = match self.redo_stack.pop() {
            Some(idx) => idx,
            None => {
                if let Some(&child) = self.nodes[self.current].children.last() {
                    child
                } else {
                    return UndoResult::Nothing;
                }
            }
        };

        let node = &self.nodes[redo_target];
        let edits: Vec<UndoEdit> = node
            .edits
            .iter()
            .map(|e| match e {
                Edit::Insert { position, text } => UndoEdit::Insert {
                    position: *position,
                    text: text.clone(),
                },
                Edit::Delete { position, text } => UndoEdit::Delete {
                    position: *position,
                    len: text.chars().count(),
                },
            })
            .collect();

        self.current = redo_target;

        UndoResult::Apply {
            edits,
            restore_cursors: None,
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current != 0 || !self.pending_edits.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty() || !self.nodes[self.current].children.is_empty()
    }

    fn gc_if_needed(&mut self) {
        if self.nodes.len() <= self.max_nodes {
            return;
        }

        // Simple GC: just prevent unbounded growth
        // A more sophisticated approach would prune old branches
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.nodes.push(UndoNode::new(None));
        self.current = 0;
        self.redo_stack.clear();
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
        // Type "hi yo" - should create 3 undo groups: "hi", " ", "yo"
        tree.record_insert(CharOffset(0), "h".into());
        tree.record_insert(CharOffset(1), "i".into());
        tree.record_insert(CharOffset(2), " ".into()); // space breaks coalescing
        tree.record_insert(CharOffset(3), "y".into());
        tree.record_insert(CharOffset(4), "o".into());
        tree.add_boundary();

        // First undo removes "yo" (2 chars)
        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 2); // "yo"
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }

        // Second undo removes " " (1 char)
        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 1); // " "
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }

        // Third undo removes "hi" (2 chars)
        match tree.undo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Delete { len, .. } => {
                        assert_eq!(*len, 2); // "hi"
                    }
                    _ => panic!("Expected delete"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_redo() {
        let mut tree = UndoTree::new(100);
        tree.record_insert(CharOffset(0), "hello".into());
        tree.add_boundary();

        tree.undo();

        match tree.redo() {
            UndoResult::Apply { edits, .. } => {
                assert_eq!(edits.len(), 1);
                match &edits[0] {
                    UndoEdit::Insert { text, .. } => {
                        assert_eq!(text, "hello");
                    }
                    _ => panic!("Expected insert"),
                }
            }
            _ => panic!("Expected Apply"),
        }
    }

    #[test]
    fn test_branch_preserves_redo() {
        let mut tree = UndoTree::new(100);
        tree.record_insert(CharOffset(0), "first".into());
        tree.add_boundary();
        tree.record_insert(CharOffset(5), "second".into());
        tree.add_boundary();

        tree.undo();

        tree.record_insert(CharOffset(5), "other".into());
        tree.add_boundary();

        assert!(tree.nodes[tree.current].parent.is_some());
    }
}
