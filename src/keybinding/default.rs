use super::key::{Key, KeyEvent, Modifiers};
use super::keymap::KeyMap;

pub fn default_keymap() -> KeyMap {
    let mut map = KeyMap::new();

    // Basic movement (Ctrl)
    map.bind_command(KeyEvent::ctrl('f'), "forward-char");
    map.bind_command(KeyEvent::ctrl('b'), "backward-char");
    map.bind_command(KeyEvent::ctrl('n'), "next-line");
    map.bind_command(KeyEvent::ctrl('p'), "previous-line");
    map.bind_command(KeyEvent::ctrl('a'), "move-beginning-of-line");
    map.bind_command(KeyEvent::ctrl('e'), "move-end-of-line");

    // Shift+Ctrl movement (selection)
    map.bind_command(KeyEvent::ctrl_shift('f'), "forward-char-shift");
    map.bind_command(KeyEvent::ctrl_shift('b'), "backward-char-shift");
    map.bind_command(KeyEvent::ctrl_shift('n'), "next-line-shift");
    map.bind_command(KeyEvent::ctrl_shift('p'), "previous-line-shift");
    map.bind_command(KeyEvent::ctrl_shift('a'), "move-beginning-of-line-shift");
    map.bind_command(KeyEvent::ctrl_shift('e'), "move-end-of-line-shift");

    // Word movement (Meta/Alt)
    map.bind_command(KeyEvent::meta('f'), "forward-word");
    map.bind_command(KeyEvent::meta('b'), "backward-word");

    // Shift+Meta word movement (selection)
    map.bind_command(KeyEvent::meta_shift('f'), "forward-word-shift");
    map.bind_command(KeyEvent::meta_shift('b'), "backward-word-shift");

    // Buffer start/end
    map.bind_command(
        KeyEvent::new(Key::Char('<'), Modifiers::META),
        "beginning-of-buffer",
    );
    map.bind_command(
        KeyEvent::new(Key::Char('>'), Modifiers::META),
        "end-of-buffer",
    );
    // Shift variants for buffer start/end
    map.bind_command(
        KeyEvent::new(Key::Char('<'), Modifiers::META | Modifiers::SHIFT),
        "beginning-of-buffer-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Char('>'), Modifiers::META | Modifiers::SHIFT),
        "end-of-buffer-shift",
    );

    map.bind_command(KeyEvent::ctrl('v'), "scroll-up-command");
    map.bind_command(KeyEvent::meta('v'), "scroll-down-command");
    map.bind_command(KeyEvent::ctrl('l'), "recenter-top-bottom");

    map.bind_command(
        KeyEvent::new(Key::Right, Modifiers::NONE),
        "forward-char",
    );
    map.bind_command(
        KeyEvent::new(Key::Left, Modifiers::NONE),
        "backward-char",
    );
    map.bind_command(
        KeyEvent::new(Key::Down, Modifiers::NONE),
        "next-line",
    );
    map.bind_command(
        KeyEvent::new(Key::Up, Modifiers::NONE),
        "previous-line",
    );
    map.bind_command(
        KeyEvent::new(Key::Home, Modifiers::NONE),
        "move-beginning-of-line",
    );
    map.bind_command(
        KeyEvent::new(Key::End, Modifiers::NONE),
        "move-end-of-line",
    );
    map.bind_command(
        KeyEvent::new(Key::PageUp, Modifiers::NONE),
        "scroll-down-command",
    );
    map.bind_command(
        KeyEvent::new(Key::PageDown, Modifiers::NONE),
        "scroll-up-command",
    );

    // Shift+Arrow for selection (shift-select mode)
    map.bind_command(
        KeyEvent::new(Key::Right, Modifiers::SHIFT),
        "forward-char-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Left, Modifiers::SHIFT),
        "backward-char-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Down, Modifiers::SHIFT),
        "next-line-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Up, Modifiers::SHIFT),
        "previous-line-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Home, Modifiers::SHIFT),
        "move-beginning-of-line-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::End, Modifiers::SHIFT),
        "move-end-of-line-shift",
    );

    // Shift+Alt+Arrow for word selection (Meta = Alt)
    map.bind_command(
        KeyEvent::new(Key::Right, Modifiers::SHIFT | Modifiers::META),
        "forward-word-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::Left, Modifiers::SHIFT | Modifiers::META),
        "backward-word-shift",
    );

    // Alt+Arrow for word movement (without selection)
    map.bind_command(
        KeyEvent::new(Key::Right, Modifiers::META),
        "forward-word",
    );
    map.bind_command(
        KeyEvent::new(Key::Left, Modifiers::META),
        "backward-word",
    );

    // Ctrl+Shift+Home/End for buffer start/end with selection
    map.bind_command(
        KeyEvent::new(Key::Home, Modifiers::CTRL | Modifiers::SHIFT),
        "beginning-of-buffer-shift",
    );
    map.bind_command(
        KeyEvent::new(Key::End, Modifiers::CTRL | Modifiers::SHIFT),
        "end-of-buffer-shift",
    );

    // Ctrl+Home/End for buffer start/end
    map.bind_command(
        KeyEvent::new(Key::Home, Modifiers::CTRL),
        "beginning-of-buffer",
    );
    map.bind_command(
        KeyEvent::new(Key::End, Modifiers::CTRL),
        "end-of-buffer",
    );

    map.bind_command(KeyEvent::ctrl('d'), "delete-char");
    map.bind_command(
        KeyEvent::new(Key::Delete, Modifiers::NONE),
        "delete-char",
    );
    map.bind_command(
        KeyEvent::new(Key::Backspace, Modifiers::NONE),
        "delete-backward-char",
    );
    map.bind_command(
        KeyEvent::new(Key::Enter, Modifiers::NONE),
        "newline",
    );
    map.bind_command(KeyEvent::ctrl('o'), "open-line");
    map.bind_command(KeyEvent::ctrl('t'), "transpose-chars");
    map.bind_command(KeyEvent::ctrl('j'), "newline");

    map.bind_command(KeyEvent::ctrl('k'), "kill-line");
    map.bind_command(KeyEvent::meta('d'), "kill-word");
    map.bind_command(
        KeyEvent::new(Key::Backspace, Modifiers::META),
        "backward-kill-word",
    );
    map.bind_command(KeyEvent::ctrl('w'), "kill-region");
    map.bind_command(KeyEvent::meta('w'), "copy-region-as-kill");
    map.bind_command(KeyEvent::ctrl('y'), "yank");
    map.bind_command(KeyEvent::meta('y'), "yank-pop");

    map.bind_command(
        KeyEvent::new(Key::Char(' '), Modifiers::CTRL),
        "set-mark-command",
    );
    map.bind_command(KeyEvent::meta('h'), "mark-paragraph");

    map.bind_command(
        KeyEvent::new(Key::Char('/'), Modifiers::CTRL),
        "undo",
    );

    // alternative keyboard layout is gay
    map.bind_command(KeyEvent::ctrl('\''), "spawn-cursors-at-word-matches");

    let mut cg_map = KeyMap::new();
    cg_map.bind_command(KeyEvent::ctrl('g'), "keyboard-quit");
    map.bind_prefix(KeyEvent::ctrl('g'), cg_map);
    map.bind_command(KeyEvent::meta('x'), "execute-extended-command");

    let mut cx_map = KeyMap::new();

    cx_map.bind_command(KeyEvent::ctrl('s'), "save-buffer");
    cx_map.bind_command(KeyEvent::ctrl('w'), "write-file");
    cx_map.bind_command(KeyEvent::ctrl('f'), "find-file");
    cx_map.bind_command(KeyEvent::char('b'), "switch-to-buffer");
    cx_map.bind_command(KeyEvent::char('k'), "kill-buffer");
    cx_map.bind_command(KeyEvent::ctrl('b'), "list-buffers");

    cx_map.bind_command(KeyEvent::char('2'), "split-window-below");
    cx_map.bind_command(KeyEvent::char('3'), "split-window-right");
    cx_map.bind_command(KeyEvent::char('0'), "delete-window");
    cx_map.bind_command(KeyEvent::char('1'), "delete-other-windows");
    cx_map.bind_command(KeyEvent::char('o'), "other-window");

    cx_map.bind_command(KeyEvent::ctrl('x'), "exchange-point-and-mark");
    cx_map.bind_command(KeyEvent::char('h'), "mark-whole-buffer");
    cx_map.bind_command(KeyEvent::char('u'), "undo");
    cx_map.bind_command(KeyEvent::char('m'), "spawn-cursors-at-word-matches");

    cx_map.bind_command(KeyEvent::ctrl('c'), "exit");

    map.bind_prefix(KeyEvent::ctrl('x'), cx_map);

    let mut mg_map = KeyMap::new();
    mg_map.bind_command(KeyEvent::char('g'), "goto-line");
    map.bind_prefix(KeyEvent::meta('g'), mg_map);

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinding::keymap::KeyBinding;

    #[test]
    fn test_default_keymap_has_basic_bindings() {
        let keymap = default_keymap();

        assert!(matches!(
            keymap.get(&KeyEvent::ctrl('f')),
            Some(KeyBinding::Command("forward-char"))
        ));

        assert!(keymap.is_prefix(&KeyEvent::ctrl('x')));

        if let Some(cx_map) = keymap.get_prefix(&KeyEvent::ctrl('x')) {
            assert!(matches!(
                cx_map.get(&KeyEvent::ctrl('s')),
                Some(KeyBinding::Command("save-buffer"))
            ));
        }
    }
}
