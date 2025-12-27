use ropey::Rope;

use super::position::{CharOffset, Position};

pub trait RopeExt {
    fn char_to_position(&self, char_idx: CharOffset) -> Position;
    fn position_to_char(&self, pos: Position) -> CharOffset;
    fn line_len_chars(&self, line_idx: usize) -> usize;
    fn total_lines(&self) -> usize;
    fn total_chars(&self) -> usize;
    fn is_word_char(c: char) -> bool;
    fn char_at(&self, char_idx: CharOffset) -> Option<char>;
    fn line_start_char(&self, line_idx: usize) -> CharOffset;
    fn line_end_char(&self, line_idx: usize) -> CharOffset;
}

impl RopeExt for Rope {
    fn char_to_position(&self, char_idx: CharOffset) -> Position {
        let len = self.len_chars();
        if len == 0 {
            return Position::new(0, 0);
        }
        let char_idx = char_idx.0.min(len);
        let line = self.char_to_line(char_idx);
        let line_start = self.line_to_char(line);
        let column = char_idx - line_start;
        Position::new(line, column)
    }

    fn position_to_char(&self, pos: Position) -> CharOffset {
        let line = pos.line.min(self.len_lines().saturating_sub(1));
        let line_start = self.line_to_char(line);
        let line_len = self.line_len_chars(line);
        let column = pos.column.min(line_len);
        CharOffset(line_start + column)
    }

    fn line_len_chars(&self, line_idx: usize) -> usize {
        if line_idx >= self.len_lines() {
            return 0;
        }
        let line = self.line(line_idx);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    fn total_lines(&self) -> usize {
        self.len_lines()
    }

    fn total_chars(&self) -> usize {
        self.len_chars()
    }

    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    fn char_at(&self, char_idx: CharOffset) -> Option<char> {
        if char_idx.0 < self.len_chars() {
            Some(self.char(char_idx.0))
        } else {
            None
        }
    }

    fn line_start_char(&self, line_idx: usize) -> CharOffset {
        CharOffset(self.line_to_char(line_idx.min(self.len_lines().saturating_sub(1))))
    }

    fn line_end_char(&self, line_idx: usize) -> CharOffset {
        let line_idx = line_idx.min(self.len_lines().saturating_sub(1));
        let start = self.line_to_char(line_idx);
        let line_len = self.line_len_chars(line_idx);
        CharOffset(start + line_len)
    }
}

pub fn find_word_boundary_forward(rope: &Rope, start: CharOffset) -> CharOffset {
    let len = rope.len_chars();
    let mut pos = start.0;

    if pos >= len {
        return CharOffset(len);
    }

    while pos < len && !Rope::is_word_char(rope.char(pos)) {
        pos += 1;
    }

    while pos < len && Rope::is_word_char(rope.char(pos)) {
        pos += 1;
    }

    CharOffset(pos)
}

pub fn find_word_boundary_backward(rope: &Rope, start: CharOffset) -> CharOffset {
    let len = rope.len_chars();
    if len == 0 {
        return CharOffset(0);
    }

    let mut pos = start.0.min(len);

    if pos == 0 {
        return CharOffset(0);
    }

    pos -= 1;

    while pos > 0 && pos < len && !Rope::is_word_char(rope.char(pos)) {
        pos -= 1;
    }

    while pos > 0 && pos < len && Rope::is_word_char(rope.char(pos - 1)) {
        pos -= 1;
    }

    if pos > 0 && pos < len && !Rope::is_word_char(rope.char(pos)) {
        CharOffset(start.0.min(len).saturating_sub(1))
    } else {
        CharOffset(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_position() {
        let rope = Rope::from_str("hello\nworld\n");
        assert_eq!(rope.char_to_position(CharOffset(0)), Position::new(0, 0));
        assert_eq!(rope.char_to_position(CharOffset(5)), Position::new(0, 5));
        assert_eq!(rope.char_to_position(CharOffset(6)), Position::new(1, 0));
        assert_eq!(rope.char_to_position(CharOffset(11)), Position::new(1, 5));
    }

    #[test]
    fn test_position_to_char() {
        let rope = Rope::from_str("hello\nworld\n");
        assert_eq!(rope.position_to_char(Position::new(0, 0)), CharOffset(0));
        assert_eq!(rope.position_to_char(Position::new(0, 5)), CharOffset(5));
        assert_eq!(rope.position_to_char(Position::new(1, 0)), CharOffset(6));
        assert_eq!(rope.position_to_char(Position::new(1, 5)), CharOffset(11));
    }

    #[test]
    fn test_word_boundary_forward() {
        let rope = Rope::from_str("hello world foo");
        assert_eq!(
            find_word_boundary_forward(&rope, CharOffset(0)),
            CharOffset(5)
        );
        assert_eq!(
            find_word_boundary_forward(&rope, CharOffset(5)),
            CharOffset(11)
        );
        assert_eq!(
            find_word_boundary_forward(&rope, CharOffset(6)),
            CharOffset(11)
        );
    }

    #[test]
    fn test_word_boundary_backward() {
        let rope = Rope::from_str("hello world foo");
        assert_eq!(
            find_word_boundary_backward(&rope, CharOffset(15)),
            CharOffset(12)
        );
        assert_eq!(
            find_word_boundary_backward(&rope, CharOffset(11)),
            CharOffset(6)
        );
        assert_eq!(
            find_word_boundary_backward(&rope, CharOffset(5)),
            CharOffset(0)
        );
    }
}
