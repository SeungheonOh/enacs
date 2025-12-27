use super::key::KeyEvent;
use super::keymap::{KeyBinding, KeyMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyResolution {
    Complete(&'static str),
    Prefix(String),
    Unbound(Vec<KeyEvent>),
    SelfInsert(char),
}

#[derive(Debug)]
pub struct KeyResolver {
    pending_keys: Vec<KeyEvent>,
    prefix_display: String,
}

impl Default for KeyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyResolver {
    pub fn new() -> Self {
        Self {
            pending_keys: Vec::new(),
            prefix_display: String::new(),
        }
    }

    pub fn resolve(&mut self, key: KeyEvent, keymap: &KeyMap) -> KeyResolution {
        self.pending_keys.push(key);

        let mut current_map = keymap;

        for (i, k) in self.pending_keys.iter().enumerate() {
            match current_map.get(k) {
                Some(KeyBinding::Command(cmd)) => {
                    if i == self.pending_keys.len() - 1 {
                        let result = KeyResolution::Complete(cmd);
                        self.clear();
                        return result;
                    }
                }
                Some(KeyBinding::Prefix(prefix_map)) => {
                    if i == self.pending_keys.len() - 1 {
                        self.update_prefix_display();
                        return KeyResolution::Prefix(self.prefix_display.clone());
                    }
                    current_map = prefix_map;
                }
                Some(KeyBinding::Unbound) | None => {
                    if i == 0 && self.pending_keys.len() == 1 {
                        if let KeyEvent {
                            key: super::key::Key::Char(c),
                            modifiers,
                        } = key
                        {
                            if modifiers == super::key::Modifiers::NONE && !c.is_control() {
                                self.clear();
                                return KeyResolution::SelfInsert(c);
                            }
                        }
                    }

                    let unbound = self.pending_keys.clone();
                    self.clear();
                    return KeyResolution::Unbound(unbound);
                }
            }
        }

        self.update_prefix_display();
        KeyResolution::Prefix(self.prefix_display.clone())
    }

    pub fn clear(&mut self) {
        self.pending_keys.clear();
        self.prefix_display.clear();
    }

    pub fn is_pending(&self) -> bool {
        !self.pending_keys.is_empty()
    }

    pub fn pending_display(&self) -> &str {
        &self.prefix_display
    }

    fn update_prefix_display(&mut self) {
        self.prefix_display = self
            .pending_keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        self.prefix_display.push('-');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinding::key::Key;

    fn make_test_keymap() -> KeyMap {
        let mut map = KeyMap::new();
        map.bind_command(KeyEvent::ctrl('f'), "forward-char");
        map.bind_command(KeyEvent::ctrl('b'), "backward-char");

        let mut cx_map = KeyMap::new();
        cx_map.bind_command(KeyEvent::ctrl('s'), "save-buffer");
        cx_map.bind_command(KeyEvent::ctrl('f'), "find-file");
        cx_map.bind_command(KeyEvent::char('b'), "switch-to-buffer");
        map.bind_prefix(KeyEvent::ctrl('x'), cx_map);

        map
    }

    #[test]
    fn test_resolve_direct_command() {
        let keymap = make_test_keymap();
        let mut resolver = KeyResolver::new();

        let result = resolver.resolve(KeyEvent::ctrl('f'), &keymap);
        assert_eq!(result, KeyResolution::Complete("forward-char"));
        assert!(!resolver.is_pending());
    }

    #[test]
    fn test_resolve_prefix_then_command() {
        let keymap = make_test_keymap();
        let mut resolver = KeyResolver::new();

        let result = resolver.resolve(KeyEvent::ctrl('x'), &keymap);
        assert!(matches!(result, KeyResolution::Prefix(_)));
        assert!(resolver.is_pending());

        let result = resolver.resolve(KeyEvent::ctrl('s'), &keymap);
        assert_eq!(result, KeyResolution::Complete("save-buffer"));
        assert!(!resolver.is_pending());
    }

    #[test]
    fn test_resolve_self_insert() {
        let keymap = make_test_keymap();
        let mut resolver = KeyResolver::new();

        let result = resolver.resolve(KeyEvent::char('a'), &keymap);
        assert_eq!(result, KeyResolution::SelfInsert('a'));
    }

    #[test]
    fn test_resolve_unbound() {
        let keymap = make_test_keymap();
        let mut resolver = KeyResolver::new();

        let result = resolver.resolve(KeyEvent::ctrl('z'), &keymap);
        assert!(matches!(result, KeyResolution::Unbound(_)));
    }

    #[test]
    fn test_resolve_unbound_after_prefix() {
        let keymap = make_test_keymap();
        let mut resolver = KeyResolver::new();

        let _ = resolver.resolve(KeyEvent::ctrl('x'), &keymap);
        let result = resolver.resolve(KeyEvent::ctrl('z'), &keymap);
        assert!(matches!(result, KeyResolution::Unbound(_)));
    }
}
