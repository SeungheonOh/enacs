use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use ropey::Rope;

use super::cursor::CursorSet;
use super::mark::MarkRing;
use super::position::CharOffset;
use super::rope_ext::RopeExt;
use super::undo::{UndoHistory, UndoResult};

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
    pub cursors: CursorSet,
    pub mark_ring: MarkRing,
    pub modified: bool,
    pub read_only: bool,
    pub mode: BufferMode,
    pub undo_history: UndoHistory,
}

impl Buffer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: BufferId::new(),
            name: name.into(),
            file_path: None,
            text: Rope::new(),
            cursors: CursorSet::new(),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_history: UndoHistory::default(),
        }
    }

    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        let mut buffer = Self {
            id: BufferId::new(),
            name,
            file_path: Some(path),
            text: Rope::from_str(&content),
            cursors: CursorSet::new(),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_history: UndoHistory::default(),
        };

        buffer.clamp_cursors();
        Ok(buffer)
    }

    pub fn from_string(name: impl Into<String>, content: impl AsRef<str>) -> Self {
        Self {
            id: BufferId::new(),
            name: name.into(),
            file_path: None,
            text: Rope::from_str(content.as_ref()),
            cursors: CursorSet::new(),
            mark_ring: MarkRing::default(),
            modified: false,
            read_only: false,
            mode: BufferMode::default(),
            undo_history: UndoHistory::default(),
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

    pub fn insert_char(&mut self, c: char) {
        self.insert_string(&c.to_string());
    }

    pub fn insert_string(&mut self, s: &str) {
        if self.read_only || s.is_empty() {
            return;
        }

        let positions = self.cursors.positions_descending();
        let char_count = s.chars().count();

        for pos in positions {
            let char_idx = pos.0.min(self.text.len_chars());
            self.undo_history.record_insert(CharOffset(char_idx), s.to_string());
            self.text.insert(char_idx, s);
            self.cursors.adjust_positions_after_insert(CharOffset(char_idx), char_count);
        }

        for cursor in self.cursors.all_cursors_mut() {
            cursor.position = CharOffset(cursor.position.0 + char_count);
            cursor.deactivate_mark();
        }

        self.modified = true;
        self.cursors.sort_and_merge();
    }

    pub fn delete_char_forward(&mut self) -> Option<char> {
        if self.read_only {
            return None;
        }

        let positions = self.cursors.positions_descending();
        let mut deleted = None;

        for pos in positions {
            let char_idx = pos.0;
            if char_idx < self.text.len_chars() {
                let c = self.text.char(char_idx);
                deleted = Some(c);
                self.undo_history.record_delete(pos, c.to_string());
                self.text.remove(char_idx..char_idx + 1);
                self.cursors.adjust_positions_after_delete(pos, CharOffset(pos.0 + 1));
            }
        }

        if deleted.is_some() {
            self.modified = true;
        }
        self.cursors.sort_and_merge();
        deleted
    }

    pub fn delete_char_backward(&mut self) -> Option<char> {
        if self.read_only {
            return None;
        }

        let positions = self.cursors.positions_descending();
        let mut deleted = None;

        for pos in positions {
            if pos.0 > 0 {
                let char_idx = pos.0 - 1;
                let c = self.text.char(char_idx);
                deleted = Some(c);
                self.undo_history.record_delete(CharOffset(char_idx), c.to_string());
                self.text.remove(char_idx..char_idx + 1);
                self.cursors.adjust_positions_after_delete(CharOffset(char_idx), pos);
            }
        }

        if deleted.is_some() {
            self.modified = true;
        }
        self.cursors.sort_and_merge();
        deleted
    }

    pub fn delete_region(&mut self, start: CharOffset, end: CharOffset) -> String {
        if self.read_only || start >= end {
            return String::new();
        }

        let start_idx = start.0.min(self.text.len_chars());
        let end_idx = end.0.min(self.text.len_chars());

        if start_idx >= end_idx {
            return String::new();
        }

        let deleted: String = self.text.slice(start_idx..end_idx).to_string();
        self.undo_history.record_delete(start, deleted.clone());
        self.text.remove(start_idx..end_idx);
        self.cursors.adjust_positions_after_delete(start, end);
        self.mark_ring.adjust_after_delete(start, end);
        self.modified = true;
        self.cursors.sort_and_merge();
        deleted
    }

    pub fn move_cursor_forward(&mut self, count: usize) {
        let max = self.text.len_chars();
        for cursor in self.cursors.all_cursors_mut() {
            cursor.position = CharOffset((cursor.position.0 + count).min(max));
            cursor.goal_column = None;
        }
    }

    pub fn move_cursor_backward(&mut self, count: usize) {
        for cursor in self.cursors.all_cursors_mut() {
            cursor.position = CharOffset(cursor.position.0.saturating_sub(count));
            cursor.goal_column = None;
        }
    }

    pub fn move_cursor_to_line_start(&mut self) {
        for cursor in self.cursors.all_cursors_mut() {
            let pos = self.text.char_to_position(cursor.position);
            cursor.position = self.text.line_start_char(pos.line);
            cursor.goal_column = Some(0);
        }
    }

    pub fn move_cursor_to_line_end(&mut self) {
        for cursor in self.cursors.all_cursors_mut() {
            let pos = self.text.char_to_position(cursor.position);
            cursor.position = self.text.line_end_char(pos.line);
            cursor.goal_column = None;
        }
    }

    pub fn move_cursor_to_next_line(&mut self) {
        let total_lines = self.text.total_lines();
        for cursor in self.cursors.all_cursors_mut() {
            let pos = self.text.char_to_position(cursor.position);
            let goal_col = cursor.goal_column.unwrap_or(pos.column);

            if pos.line + 1 < total_lines {
                let next_line = pos.line + 1;
                let line_len = self.text.line_len_chars(next_line);
                let new_col = goal_col.min(line_len);
                let line_start = self.text.line_start_char(next_line);
                cursor.position = CharOffset(line_start.0 + new_col);
                cursor.goal_column = Some(goal_col);
            }
        }
    }

    pub fn move_cursor_to_prev_line(&mut self) {
        for cursor in self.cursors.all_cursors_mut() {
            let pos = self.text.char_to_position(cursor.position);
            let goal_col = cursor.goal_column.unwrap_or(pos.column);

            if pos.line > 0 {
                let prev_line = pos.line - 1;
                let line_len = self.text.line_len_chars(prev_line);
                let new_col = goal_col.min(line_len);
                let line_start = self.text.line_start_char(prev_line);
                cursor.position = CharOffset(line_start.0 + new_col);
                cursor.goal_column = Some(goal_col);
            }
        }
    }

    pub fn move_cursor_to_buffer_start(&mut self) {
        for cursor in self.cursors.all_cursors_mut() {
            cursor.position = CharOffset(0);
            cursor.goal_column = None;
        }
    }

    pub fn move_cursor_to_buffer_end(&mut self) {
        let end = self.text.len_chars();
        for cursor in self.cursors.all_cursors_mut() {
            cursor.position = CharOffset(end);
            cursor.goal_column = None;
        }
    }

    pub fn clamp_cursors(&mut self) {
        let max = self.text.len_chars();
        for cursor in self.cursors.all_cursors_mut() {
            if cursor.position.0 > max {
                cursor.position = CharOffset(max);
            }
            if let Some(mark) = cursor.mark {
                if mark.0 > max {
                    cursor.mark = Some(CharOffset(max));
                }
            }
        }
    }

    pub fn undo(&mut self) -> bool {
        self.undo_history.start_undo_sequence();

        match self.undo_history.undo_one() {
            UndoResult::Insert { position, text } => {
                let char_idx = position.0.min(self.text.len_chars());
                self.text.insert(char_idx, &text);
                let len = text.chars().count();
                self.cursors.adjust_positions_after_insert(position, len);
                self.cursors.primary.position = CharOffset(char_idx + len);
                self.modified = true;
                true
            }
            UndoResult::Delete { position, len } => {
                let start = position.0.min(self.text.len_chars());
                let end = (start + len).min(self.text.len_chars());
                if start < end {
                    self.text.remove(start..end);
                    self.cursors.adjust_positions_after_delete(position, CharOffset(end));
                    self.cursors.primary.position = position;
                    self.modified = true;
                }
                true
            }
            UndoResult::RestoreCursors(cursors) => {
                self.cursors = cursors;
                true
            }
            UndoResult::Nothing => false,
        }
    }

    pub fn add_undo_boundary(&mut self) {
        self.undo_history.add_boundary();
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
        buffer.insert_string("hello");
        assert_eq!(buffer.text.to_string(), "hello");
        assert_eq!(buffer.cursors.primary.position, CharOffset(5));
    }

    #[test]
    fn test_buffer_delete_forward() {
        let mut buffer = Buffer::from_string("test", "hello");
        buffer.cursors.primary.position = CharOffset(0);
        buffer.delete_char_forward();
        assert_eq!(buffer.text.to_string(), "ello");
    }

    #[test]
    fn test_buffer_delete_backward() {
        let mut buffer = Buffer::from_string("test", "hello");
        buffer.cursors.primary.position = CharOffset(5);
        buffer.delete_char_backward();
        assert_eq!(buffer.text.to_string(), "hell");
        assert_eq!(buffer.cursors.primary.position, CharOffset(4));
    }

    #[test]
    fn test_buffer_movement() {
        let mut buffer = Buffer::from_string("test", "hello\nworld\n");
        buffer.cursors.primary.position = CharOffset(0);

        buffer.move_cursor_to_line_end();
        assert_eq!(buffer.cursors.primary.position, CharOffset(5));

        buffer.move_cursor_to_next_line();
        assert_eq!(buffer.cursors.primary.position, CharOffset(11));

        buffer.move_cursor_to_line_start();
        assert_eq!(buffer.cursors.primary.position, CharOffset(6));
    }

    #[test]
    fn test_buffer_undo() {
        let mut buffer = Buffer::new("test");
        buffer.insert_string("hello");
        buffer.add_undo_boundary();
        buffer.insert_string(" world");

        assert_eq!(buffer.text.to_string(), "hello world");

        buffer.undo();
        assert_eq!(buffer.text.to_string(), "hello");
    }

    #[test]
    fn test_multi_cursor_insert() {
        let mut buffer = Buffer::from_string("test", "aa bb cc");
        buffer.cursors.primary.position = CharOffset(0);
        buffer.cursors.add_cursor(CharOffset(3));
        buffer.cursors.add_cursor(CharOffset(6));

        buffer.insert_string("X");

        assert_eq!(buffer.text.to_string(), "Xaa Xbb Xcc");
    }
}
