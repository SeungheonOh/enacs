use bitflags::bitflags;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    F(u8),
    Backspace,
    Tab,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const NONE  = 0b0000;
        const CTRL  = 0b0001;
        const META  = 0b0010;
        const SHIFT = 0b0100;
        const SUPER = 0b1000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    pub fn char(c: char) -> Self {
        Self {
            key: Key::Char(c),
            modifiers: Modifiers::NONE,
        }
    }

    pub fn ctrl(c: char) -> Self {
        Self {
            key: Key::Char(c.to_ascii_lowercase()),
            modifiers: Modifiers::CTRL,
        }
    }

    pub fn meta(c: char) -> Self {
        Self {
            key: Key::Char(c.to_ascii_lowercase()),
            modifiers: Modifiers::META,
        }
    }

    pub fn ctrl_meta(c: char) -> Self {
        Self {
            key: Key::Char(c.to_ascii_lowercase()),
            modifiers: Modifiers::CTRL | Modifiers::META,
        }
    }

    pub fn ctrl_shift(c: char) -> Self {
        Self {
            key: Key::Char(c.to_ascii_lowercase()),
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        }
    }

    pub fn meta_shift(c: char) -> Self {
        Self {
            key: Key::Char(c.to_ascii_lowercase()),
            modifiers: Modifiers::META | Modifiers::SHIFT,
        }
    }

    pub fn ctrl_key(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    pub fn meta_key(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::META,
        }
    }

    pub fn is_printable(&self) -> bool {
        matches!(self.key, Key::Char(c) if !c.is_control())
            && self.modifiers == Modifiers::NONE
    }
}

impl fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(Modifiers::CTRL) {
            write!(f, "C-")?;
        }
        if self.modifiers.contains(Modifiers::META) {
            write!(f, "M-")?;
        }
        if self.modifiers.contains(Modifiers::SUPER) {
            write!(f, "s-")?;
        }
        if self.modifiers.contains(Modifiers::SHIFT) {
            write!(f, "S-")?;
        }

        match self.key {
            Key::Char(c) => write!(f, "{}", c),
            Key::F(n) => write!(f, "<f{}>", n),
            Key::Backspace => write!(f, "<backspace>"),
            Key::Tab => write!(f, "<tab>"),
            Key::Enter => write!(f, "<return>"),
            Key::Escape => write!(f, "<escape>"),
            Key::Up => write!(f, "<up>"),
            Key::Down => write!(f, "<down>"),
            Key::Left => write!(f, "<left>"),
            Key::Right => write!(f, "<right>"),
            Key::Home => write!(f, "<home>"),
            Key::End => write!(f, "<end>"),
            Key::PageUp => write!(f, "<prior>"),
            Key::PageDown => write!(f, "<next>"),
            Key::Insert => write!(f, "<insert>"),
            Key::Delete => write!(f, "<delete>"),
        }
    }
}

impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(event: crossterm::event::KeyEvent) -> Self {
        use crossterm::event::{KeyCode, KeyModifiers};

        let mut modifiers = Modifiers::NONE;
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            modifiers |= Modifiers::CTRL;
        }
        if event.modifiers.contains(KeyModifiers::ALT) {
            modifiers |= Modifiers::META;
        }
        if event.modifiers.contains(KeyModifiers::SHIFT) {
            modifiers |= Modifiers::SHIFT;
        }
        if event.modifiers.contains(KeyModifiers::SUPER) {
            modifiers |= Modifiers::SUPER;
        }

        let key = match event.code {
            KeyCode::Char(c) => {
                // Normalize: if char is uppercase and we have modifiers (Ctrl/Alt),
                // convert to lowercase and add SHIFT modifier.
                // This handles terminals that send 'B' with Alt instead of 'b' with Alt+Shift.
                if c.is_ascii_uppercase() && (modifiers.contains(Modifiers::CTRL) || modifiers.contains(Modifiers::META)) {
                    modifiers |= Modifiers::SHIFT;
                    Key::Char(c.to_ascii_lowercase())
                } else {
                    Key::Char(c)
                }
            }
            KeyCode::F(n) => Key::F(n),
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Tab => Key::Tab,
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::Escape,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Insert => Key::Insert,
            KeyCode::Delete => Key::Delete,
            _ => Key::Char('\0'),
        };

        Self { key, modifiers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_display() {
        assert_eq!(KeyEvent::ctrl('x').to_string(), "C-x");
        assert_eq!(KeyEvent::meta('x').to_string(), "M-x");
        assert_eq!(KeyEvent::ctrl_meta('x').to_string(), "C-M-x");
        assert_eq!(KeyEvent::char('a').to_string(), "a");
    }

    #[test]
    fn test_is_printable() {
        assert!(KeyEvent::char('a').is_printable());
        assert!(!KeyEvent::ctrl('a').is_printable());
        assert!(!KeyEvent::new(Key::Backspace, Modifiers::NONE).is_printable());
    }
}
