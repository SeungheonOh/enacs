use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct ByteOffset(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct CharOffset(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub fn start() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.line.cmp(&other.line) {
            Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

impl ByteOffset {
    pub fn saturating_sub(self, rhs: usize) -> Self {
        ByteOffset(self.0.saturating_sub(rhs))
    }

    pub fn saturating_add(self, rhs: usize) -> Self {
        ByteOffset(self.0.saturating_add(rhs))
    }
}

impl CharOffset {
    pub fn saturating_sub(self, rhs: usize) -> Self {
        CharOffset(self.0.saturating_sub(rhs))
    }

    pub fn saturating_add(self, rhs: usize) -> Self {
        CharOffset(self.0.saturating_add(rhs))
    }
}

impl From<usize> for CharOffset {
    fn from(n: usize) -> Self {
        CharOffset(n)
    }
}

impl From<usize> for ByteOffset {
    fn from(n: usize) -> Self {
        ByteOffset(n)
    }
}

impl From<CharOffset> for usize {
    fn from(offset: CharOffset) -> Self {
        offset.0
    }
}

impl From<ByteOffset> for usize {
    fn from(offset: ByteOffset) -> Self {
        offset.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_ordering() {
        let p1 = Position::new(0, 5);
        let p2 = Position::new(1, 0);
        let p3 = Position::new(1, 5);

        assert!(p1 < p2);
        assert!(p2 < p3);
        assert!(p1 < p3);
    }
}
