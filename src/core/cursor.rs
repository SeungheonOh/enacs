use super::position::CharOffset;

#[derive(Debug, Clone)]
pub struct Cursor {
    pub position: CharOffset,
    pub goal_column: Option<usize>,
    pub mark: Option<CharOffset>,
    pub mark_active: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            position: CharOffset(0),
            goal_column: None,
            mark: None,
            mark_active: false,
        }
    }
}

impl Cursor {
    pub fn new(position: CharOffset) -> Self {
        Self {
            position,
            goal_column: None,
            mark: None,
            mark_active: false,
        }
    }

    pub fn set_position(&mut self, pos: CharOffset) {
        self.position = pos;
        self.goal_column = None;
    }

    pub fn set_mark(&mut self, pos: CharOffset) {
        self.mark = Some(pos);
        self.mark_active = true;
    }

    pub fn deactivate_mark(&mut self) {
        self.mark_active = false;
    }

    pub fn clear_mark(&mut self) {
        self.mark = None;
        self.mark_active = false;
    }

    pub fn region(&self) -> Option<(CharOffset, CharOffset)> {
        if self.mark_active {
            self.mark.map(|mark| {
                if mark < self.position {
                    (mark, self.position)
                } else {
                    (self.position, mark)
                }
            })
        } else {
            None
        }
    }

    pub fn region_or_point(&self) -> (CharOffset, CharOffset) {
        self.region()
            .unwrap_or((self.position, self.position))
    }

    pub fn exchange_point_and_mark(&mut self) {
        if let Some(mark) = self.mark {
            let old_pos = self.position;
            self.position = mark;
            self.mark = Some(old_pos);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CursorSet {
    pub primary: Cursor,
    pub secondary: Vec<Cursor>,
}

impl Default for CursorSet {
    fn default() -> Self {
        Self {
            primary: Cursor::default(),
            secondary: Vec::new(),
        }
    }
}

impl CursorSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn single(position: CharOffset) -> Self {
        Self {
            primary: Cursor::new(position),
            secondary: Vec::new(),
        }
    }

    pub fn all_cursors(&self) -> impl Iterator<Item = &Cursor> {
        std::iter::once(&self.primary).chain(self.secondary.iter())
    }

    pub fn all_cursors_mut(&mut self) -> impl Iterator<Item = &mut Cursor> {
        std::iter::once(&mut self.primary).chain(self.secondary.iter_mut())
    }

    pub fn count(&self) -> usize {
        1 + self.secondary.len()
    }

    pub fn add_cursor(&mut self, position: CharOffset) {
        for cursor in self.all_cursors() {
            if cursor.position == position {
                return;
            }
        }

        let cursor = Cursor::new(position);
        self.secondary.push(cursor);
        self.sort_and_merge();
    }

    pub fn remove_secondary_cursors(&mut self) {
        self.secondary.clear();
    }

    pub fn sort_and_merge(&mut self) {
        if self.secondary.is_empty() {
            return;
        }

        let mut all: Vec<Cursor> = std::iter::once(self.primary.clone())
            .chain(self.secondary.drain(..))
            .collect();

        all.sort_by_key(|c| c.position);
        all.dedup_by_key(|c| c.position);

        self.primary = all.remove(0);
        self.secondary = all;
    }

    pub fn positions_descending(&self) -> Vec<CharOffset> {
        let mut positions: Vec<_> = self.all_cursors().map(|c| c.position).collect();
        positions.sort();
        positions.reverse();
        positions
    }

    pub fn adjust_positions_after_insert(&mut self, insert_pos: CharOffset, len: usize) {
        for cursor in self.all_cursors_mut() {
            if cursor.position > insert_pos {
                cursor.position = CharOffset(cursor.position.0 + len);
            }
            if let Some(mark) = cursor.mark {
                if mark > insert_pos {
                    cursor.mark = Some(CharOffset(mark.0 + len));
                }
            }
        }
    }

    pub fn adjust_positions_after_delete(&mut self, delete_start: CharOffset, delete_end: CharOffset) {
        let deleted_len = delete_end.0 - delete_start.0;
        for cursor in self.all_cursors_mut() {
            if cursor.position >= delete_end {
                cursor.position = CharOffset(cursor.position.0 - deleted_len);
            } else if cursor.position > delete_start {
                cursor.position = delete_start;
            }
            if let Some(mark) = cursor.mark {
                if mark >= delete_end {
                    cursor.mark = Some(CharOffset(mark.0 - deleted_len));
                } else if mark > delete_start {
                    cursor.mark = Some(delete_start);
                }
            }
        }
    }

    pub fn deactivate_all_marks(&mut self) {
        for cursor in self.all_cursors_mut() {
            cursor.deactivate_mark();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_region() {
        let mut cursor = Cursor::new(CharOffset(10));
        cursor.set_mark(CharOffset(5));
        assert_eq!(cursor.region(), Some((CharOffset(5), CharOffset(10))));

        cursor.deactivate_mark();
        assert_eq!(cursor.region(), None);
    }

    #[test]
    fn test_cursor_set_add_and_merge() {
        let mut set = CursorSet::single(CharOffset(10));
        set.add_cursor(CharOffset(5));
        set.add_cursor(CharOffset(15));
        set.add_cursor(CharOffset(10));

        assert_eq!(set.count(), 3);

        let positions: Vec<_> = set.all_cursors().map(|c| c.position.0).collect();
        assert_eq!(positions, vec![5, 10, 15]);
    }

    #[test]
    fn test_adjust_after_insert() {
        let mut set = CursorSet::single(CharOffset(10));
        set.add_cursor(CharOffset(20));
        set.add_cursor(CharOffset(30));

        set.adjust_positions_after_insert(CharOffset(15), 5);

        let positions: Vec<_> = set.all_cursors().map(|c| c.position.0).collect();
        assert_eq!(positions, vec![10, 25, 35]);
    }

    #[test]
    fn test_adjust_after_delete() {
        let mut set = CursorSet::single(CharOffset(10));
        set.add_cursor(CharOffset(20));
        set.add_cursor(CharOffset(30));

        set.adjust_positions_after_delete(CharOffset(12), CharOffset(18));

        let positions: Vec<_> = set.all_cursors().map(|c| c.position.0).collect();
        assert_eq!(positions, vec![10, 14, 24]);
    }
}
