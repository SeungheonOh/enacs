use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ropey::Rope;

use super::cursor::{CursorId, CursorSet};
use super::mark::MarkRing;
use super::position::CharOffset;
use super::undo::{UndoEdit, UndoResult, UndoTree};

static BUFFER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

impl BufferId {
    pub fn new() -> Self {
        Self(BUFFER_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for BufferId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferMode {
    #[default]
    Fundamental,
    Text,
    ReadOnly,
}

#[derive(Debug)]
pub struct Buffer {
    pub id: BufferId,
    pub name: String,
    pub file_path: Option<PathBuf>,
    pub text: Rope,
    pub mark_ring: MarkRing,
    pub modified: bool,
    pub read_only: bool,
    pub mode: BufferMode,
    pub undo_tree: UndoTree,
}

impl Buffer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: BufferId::new(),
            name: name.into(),
            file_path: None,
            text: Rope::new(),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_tree: UndoTree::default(),
        }
    }

    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let buffer = Self {
            id: BufferId::new(),
            name,
            file_path: Some(path),
            text: Rope::from_str(&content),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_tree: UndoTree::default(),
        };

        Ok(buffer)
    }

    pub fn from_string(name: impl Into<String>, content: impl AsRef<str>) -> Self {
        Self {
            id: BufferId::new(),
            name: name.into(),
            file_path: None,
            text: Rope::from_str(content.as_ref()),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_tree: UndoTree::default(),
        }
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            std::fs::write(path, self.text.to_string())?;
            self.modified = false;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Buffer has no file path",
            ))
        }
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        std::fs::write(&path, self.text.to_string())?;
        self.name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        self.file_path = Some(path);
        self.modified = false;
        Ok(())
    }

    pub fn insert_char(&mut self, cursors: &mut CursorSet, c: char) {
        self.insert_string(cursors, &c.to_string());
    }

    pub fn insert_string(&mut self, cursors: &mut CursorSet, s: &str) {
        if self.read_only || s.is_empty() {
            return;
        }

        let is_newline = s.contains('\n');
        if is_newline {
            self.undo_tree.add_boundary();
        }

        self.undo_tree.set_cursors_before(cursors.clone());

        let positions = cursors.positions_descending();
        let char_count = s.chars().count();

        self.undo_tree.begin_batch();

        for pos in positions {
            let char_idx = pos.0.min(self.text.len_chars());
            self.undo_tree
                .record_insert(CharOffset(char_idx), s.to_string());
            self.text.insert(char_idx, s);
            cursors.adjust_positions_after_insert(CharOffset(char_idx), char_count);
        }

        self.undo_tree.end_batch();

        for cursor in cursors.all_cursors_mut() {
            cursor.position = CharOffset(cursor.position.0 + char_count);
            cursor.deactivate_mark();
        }

        self.modified = true;
        cursors.sort();

        if is_newline {
            self.undo_tree.add_boundary();
        }
    }

    pub fn insert_at_cursors(&mut self, cursors: &mut CursorSet, texts: Vec<(CursorId, String)>) {
        if self.read_only || texts.is_empty() {
            return;
        }

        let mut ops: Vec<(CursorId, CharOffset, String)> = texts
            .into_iter()
            .filter_map(|(id, text)| {
                if text.is_empty() {
                    return None;
                }
                cursors.get_by_id(id).map(|c| (id, c.position, text))
            })
            .collect();

        ops.sort_by(|a, b| b.1.cmp(&a.1));

        self.undo_tree.begin_batch();

        for (cursor_id, pos, text) in ops {
            let char_idx = pos.0.min(self.text.len_chars());
            let char_count = text.chars().count();

            self.undo_tree
                .record_insert(CharOffset(char_idx), text.clone());
            self.text.insert(char_idx, &text);

            for cursor in cursors.all_cursors_mut() {
                if cursor.id == cursor_id {
                    cursor.position = CharOffset(char_idx + char_count);
                    cursor.deactivate_mark();
                } else if cursor.position > CharOffset(char_idx) {
                    cursor.position = CharOffset(cursor.position.0 + char_count);
                }
                if let Some(mark) = cursor.mark {
                    if mark > CharOffset(char_idx) {
                        cursor.mark = Some(CharOffset(mark.0 + char_count));
                    }
                }
            }
        }

        self.undo_tree.end_batch();

        self.modified = true;
        cursors.sort();
    }

    pub fn delete_char_forward(&mut self, cursors: &mut CursorSet) -> Option<char> {
        if self.read_only {
            return None;
        }

        let positions = cursors.positions_descending();
        let mut deleted = None;

        self.undo_tree.begin_batch();

        for pos in positions {
            let char_idx = pos.0;
            if char_idx < self.text.len_chars() {
                let c = self.text.char(char_idx);
                deleted = Some(c);
                self.undo_tree.record_delete(pos, c.to_string());
                self.text.remove(char_idx..char_idx + 1);
                cursors.adjust_positions_after_delete(pos, CharOffset(pos.0 + 1));
            }
        }

        self.undo_tree.end_batch();

        if deleted.is_some() {
            self.modified = true;
        }
        cursors.sort();
        deleted
    }

    pub fn delete_char_backward(&mut self, cursors: &mut CursorSet) -> Option<char> {
        if self.read_only {
            return None;
        }

        let positions = cursors.positions_descending();
        let mut deleted = None;

        self.undo_tree.begin_batch();

        for pos in positions {
            if pos.0 > 0 {
                let char_idx = pos.0 - 1;
                let c = self.text.char(char_idx);
                deleted = Some(c);
                self.undo_tree
                    .record_delete(CharOffset(char_idx), c.to_string());
                self.text.remove(char_idx..char_idx + 1);
                cursors.adjust_positions_after_delete(CharOffset(char_idx), pos);
            }
        }

        self.undo_tree.end_batch();

        if deleted.is_some() {
            self.modified = true;
        }
        cursors.sort();
        deleted
    }

    pub fn delete_region(
        &mut self,
        cursors: &mut CursorSet,
        start: CharOffset,
        end: CharOffset,
    ) -> String {
        if self.read_only || start >= end {
            return String::new();
        }

        let start_idx = start.0.min(self.text.len_chars());
        let end_idx = end.0.min(self.text.len_chars());

        if start_idx >= end_idx {
            return String::new();
        }

        self.undo_tree.break_coalesce();

        let deleted: String = self.text.slice(start_idx..end_idx).to_string();
        self.undo_tree.record_delete(start, deleted.clone());
        self.text.remove(start_idx..end_idx);
        cursors.adjust_positions_after_delete(start, end);
        self.mark_ring.adjust_after_delete(start, end);
        self.modified = true;
        cursors.sort();

        self.undo_tree.break_coalesce();

        deleted
    }

    pub fn delete_regions(
        &mut self,
        cursors: &mut CursorSet,
        regions: Vec<(CursorId, CharOffset, CharOffset)>,
    ) -> Vec<(CursorId, String)> {
        if self.read_only {
            return Vec::new();
        }

        let mut ops: Vec<(CursorId, CharOffset, CharOffset)> = regions
            .into_iter()
            .filter(|(_, start, end)| start < end)
            .collect();

        ops.sort_by(|a, b| b.1.cmp(&a.1));

        self.undo_tree.break_coalesce();
        self.undo_tree.begin_batch();

        let mut results = Vec::new();

        for (cursor_id, start, end) in ops {
            let start_idx = start.0.min(self.text.len_chars());
            let end_idx = end.0.min(self.text.len_chars());

            if start_idx >= end_idx {
                continue;
            }

            let deleted: String = self.text.slice(start_idx..end_idx).to_string();
            self.undo_tree.record_delete(start, deleted.clone());
            self.text.remove(start_idx..end_idx);
            cursors.adjust_positions_after_delete(start, end);
            self.mark_ring.adjust_after_delete(start, end);

            results.push((cursor_id, deleted));
        }

        self.undo_tree.end_batch();

        self.modified = true;
        cursors.sort();

        results
    }

    fn apply_undo_edits(&mut self, cursors: &mut CursorSet, edits: Vec<UndoEdit>) {
        for edit in edits {
            match edit {
                UndoEdit::Insert { position, text } => {
                    let char_idx = position.0.min(self.text.len_chars());
                    self.text.insert(char_idx, &text);
                    let len = text.chars().count();
                    cursors.adjust_positions_after_insert(position, len);
                    cursors.primary.position = CharOffset(char_idx + len);
                }
                UndoEdit::Delete { position, len } => {
                    let start = position.0.min(self.text.len_chars());
                    let end = (start + len).min(self.text.len_chars());
                    if start < end {
                        self.text.remove(start..end);
                        cursors.adjust_positions_after_delete(position, CharOffset(end));
                        cursors.primary.position = position;
                    }
                }
            }
        }
        self.modified = true;
    }

    pub fn undo(&mut self, cursors: &mut CursorSet) -> bool {
        match self.undo_tree.undo() {
            UndoResult::Apply {
                edits,
                restore_cursors,
            } => {
                self.apply_undo_edits(cursors, edits);
                if let Some(saved_cursors) = restore_cursors {
                    *cursors = saved_cursors;
                }
                true
            }
            UndoResult::Nothing => false,
        }
    }

    pub fn redo(&mut self, cursors: &mut CursorSet) -> bool {
        match self.undo_tree.redo() {
            UndoResult::Apply {
                edits,
                restore_cursors,
            } => {
                self.apply_undo_edits(cursors, edits);
                if let Some(saved_cursors) = restore_cursors {
                    *cursors = saved_cursors;
                }
                true
            }
            UndoResult::Nothing => false,
        }
    }

    pub fn add_undo_boundary(&mut self) {
        self.undo_tree.add_boundary();
    }

    pub fn break_undo_coalesce(&mut self) {
        self.undo_tree.break_coalesce();
    }

    pub fn set_undo_cursors(&mut self, cursors: &CursorSet) {
        self.undo_tree.set_cursors_before(cursors.clone());
    }

    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.text.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.text.len_chars() == 0
    }

    pub fn line(&self, line_idx: usize) -> Option<ropey::RopeSlice<'_>> {
        if line_idx < self.text.len_lines() {
            Some(self.text.line(line_idx))
        } else {
            None
        }
    }

    pub fn slice(&self, start: CharOffset, end: CharOffset) -> String {
        let start_idx = start.0.min(self.text.len_chars());
        let end_idx = end.0.min(self.text.len_chars());
        if start_idx < end_idx {
            self.text.slice(start_idx..end_idx).to_string()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_insert() {
        let mut buffer = Buffer::new("test");
        let mut cursors = CursorSet::new();
        buffer.insert_string(&mut cursors, "hello");
        assert_eq!(buffer.text.to_string(), "hello");
        assert_eq!(cursors.primary.position, CharOffset(5));
    }

    #[test]
    fn test_buffer_delete_forward() {
        let mut buffer = Buffer::from_string("test", "hello");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(0);
        buffer.delete_char_forward(&mut cursors);
        assert_eq!(buffer.text.to_string(), "ello");
    }

    #[test]
    fn test_buffer_delete_backward() {
        let mut buffer = Buffer::from_string("test", "hello");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(5);
        buffer.delete_char_backward(&mut cursors);
        assert_eq!(buffer.text.to_string(), "hell");
        assert_eq!(cursors.primary.position, CharOffset(4));
    }

    #[test]
    fn test_buffer_undo() {
        let mut buffer = Buffer::new("test");
        let mut cursors = CursorSet::new();
        buffer.insert_string(&mut cursors, "hello");
        buffer.add_undo_boundary();
        buffer.insert_string(&mut cursors, " world");

        assert_eq!(buffer.text.to_string(), "hello world");

        buffer.undo(&mut cursors);
        assert_eq!(buffer.text.to_string(), "hello");
    }

    #[test]
    fn test_multi_cursor_insert() {
        let mut buffer = Buffer::from_string("test", "aa bb cc");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(0);
        cursors.add_cursor(CharOffset(3));
        cursors.add_cursor(CharOffset(6));

        buffer.insert_string(&mut cursors, "X");

        assert_eq!(buffer.text.to_string(), "Xaa Xbb Xcc");
    }

    #[test]
    fn test_multi_cursor_undo_single_action() {
        let mut buffer = Buffer::from_string("test", "aa bb cc");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(0);
        cursors.add_cursor(CharOffset(3));
        cursors.add_cursor(CharOffset(6));

        buffer.insert_string(&mut cursors, "X");
        assert_eq!(buffer.text.to_string(), "Xaa Xbb Xcc");

        buffer.add_undo_boundary();

        buffer.undo(&mut cursors);
        assert_eq!(buffer.text.to_string(), "aa bb cc");
    }

    #[test]
    fn test_multi_cursor_coalesce_word() {
        let mut buffer = Buffer::from_string("test", "");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(0);

        buffer.insert_string(&mut cursors, "h");
        buffer.insert_string(&mut cursors, "e");
        buffer.insert_string(&mut cursors, "l");
        buffer.insert_string(&mut cursors, "l");
        buffer.insert_string(&mut cursors, "o");

        assert_eq!(buffer.text.to_string(), "hello");

        buffer.add_undo_boundary();
        buffer.undo(&mut cursors);

        assert_eq!(buffer.text.to_string(), "");
    }

    #[test]
    fn test_undo_newline_cursor_position() {
        let mut buffer = Buffer::new("test");
        let mut cursors = CursorSet::new();

        // 1. Type "foo"
        buffer.insert_string(&mut cursors, "foo");
        assert_eq!(buffer.text.to_string(), "foo");
        assert_eq!(cursors.primary.position, CharOffset(3));

        // 2. Type newline
        buffer.insert_string(&mut cursors, "\n");
        assert_eq!(buffer.text.to_string(), "foo\n");
        assert_eq!(cursors.primary.position, CharOffset(4));

        // 3. Type "bar"
        buffer.insert_string(&mut cursors, "bar");
        assert_eq!(buffer.text.to_string(), "foo\nbar");
        assert_eq!(cursors.primary.position, CharOffset(7));

        // Undo "bar"
        buffer.undo(&mut cursors);
        assert_eq!(buffer.text.to_string(), "foo\n");
        // Cursor should be at start of line 2 (offset 4)
        assert_eq!(cursors.primary.position, CharOffset(4));

        // Undo "\n"
        buffer.undo(&mut cursors);
        assert_eq!(buffer.text.to_string(), "foo");
        // Cursor should be at end of line 1 (offset 3)
        assert_eq!(cursors.primary.position, CharOffset(3));

        // Undo "foo"
        buffer.undo(&mut cursors);
        assert_eq!(buffer.text.to_string(), "");
        assert_eq!(cursors.primary.position, CharOffset(0));
    }

    #[test]
    fn test_multi_cursor_coalesce_word_multiple_cursors() {
        let mut buffer = Buffer::from_string("test", "X Y");
        let mut cursors = CursorSet::new();
        cursors.primary.position = CharOffset(0);
        cursors.add_cursor(CharOffset(2));

        buffer.insert_string(&mut cursors, "h");
        assert_eq!(buffer.text.to_string(), "hX hY");

        buffer.insert_string(&mut cursors, "i");
        assert_eq!(buffer.text.to_string(), "hiX hiY");

        buffer.add_undo_boundary();
        buffer.undo(&mut cursors);

        assert_eq!(buffer.text.to_string(), "X Y");
    }
}
