use std::collections::HashMap;
use super::key::KeyEvent;

#[derive(Debug, Clone)]
pub enum KeyBinding {
    Command(&'static str),
    Prefix(KeyMap),
    Unbound,
}

#[derive(Debug, Clone, Default)]
pub struct KeyMap {
    bindings: HashMap<KeyEvent, KeyBinding>,
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, key: KeyEvent, binding: KeyBinding) {
        self.bindings.insert(key, binding);
    }

    pub fn bind_command(&mut self, key: KeyEvent, command: &'static str) {
        self.bind(key, KeyBinding::Command(command));
    }

    pub fn bind_prefix(&mut self, key: KeyEvent, map: KeyMap) {
        self.bind(key, KeyBinding::Prefix(map));
    }

    pub fn get(&self, key: &KeyEvent) -> Option<&KeyBinding> {
        self.bindings.get(key)
    }

    pub fn get_or_unbound(&self, key: &KeyEvent) -> &KeyBinding {
        self.bindings.get(key).unwrap_or(&KeyBinding::Unbound)
    }

    pub fn unbind(&mut self, key: &KeyEvent) {
        self.bindings.remove(key);
    }

    pub fn is_prefix(&self, key: &KeyEvent) -> bool {
        matches!(self.bindings.get(key), Some(KeyBinding::Prefix(_)))
    }

    pub fn get_prefix(&self, key: &KeyEvent) -> Option<&KeyMap> {
        match self.bindings.get(key) {
            Some(KeyBinding::Prefix(map)) => Some(map),
            _ => None,
        }
    }

    pub fn get_prefix_mut(&mut self, key: &KeyEvent) -> Option<&mut KeyMap> {
        match self.bindings.get_mut(key) {
            Some(KeyBinding::Prefix(map)) => Some(map),
            _ => None,
        }
    }

    pub fn ensure_prefix(&mut self, key: KeyEvent) -> &mut KeyMap {
        if !self.is_prefix(&key) {
            self.bind(key, KeyBinding::Prefix(KeyMap::new()));
        }
        self.get_prefix_mut(&key).unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&KeyEvent, &KeyBinding)> {
        self.bindings.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinding::key::Key;

    #[test]
    fn test_bind_command() {
        let mut map = KeyMap::new();
        map.bind_command(KeyEvent::ctrl('f'), "forward-char");

        match map.get(&KeyEvent::ctrl('f')) {
            Some(KeyBinding::Command(name)) => assert_eq!(*name, "forward-char"),
            _ => panic!("Expected command binding"),
        }
    }

    #[test]
    fn test_bind_prefix() {
        let mut map = KeyMap::new();
        let mut prefix = KeyMap::new();
        prefix.bind_command(KeyEvent::ctrl('s'), "save-buffer");
        map.bind_prefix(KeyEvent::ctrl('x'), prefix);

        assert!(map.is_prefix(&KeyEvent::ctrl('x')));

        if let Some(prefix_map) = map.get_prefix(&KeyEvent::ctrl('x')) {
            match prefix_map.get(&KeyEvent::ctrl('s')) {
                Some(KeyBinding::Command(name)) => assert_eq!(*name, "save-buffer"),
                _ => panic!("Expected command binding"),
            }
        } else {
            panic!("Expected prefix map");
        }
    }
}
