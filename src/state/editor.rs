use std::path::PathBuf;

use crate::commands::registry::{build_default_registry, CommandContext, CommandRegistry, PrefixArg};
use crate::core::{Buffer, BufferId, KillRing};
use crate::keybinding::default::default_keymap;
use crate::keybinding::{KeyEvent, KeyMap, KeyResolution, KeyResolver};

use super::buffer_mgr::BufferManager;
use super::minibuffer::Minibuffer;
use super::window_mgr::{Window, WindowManager};

pub struct EditorState {
    pub buffers: BufferManager,
    pub windows: WindowManager,
    pub minibuffer: Minibuffer,
    pub kill_ring: KillRing,
    pub keymap: KeyMap,
    pub key_resolver: KeyResolver,
    pub command_registry: CommandRegistry,
    pub message: Option<String>,
    pub last_command: Option<&'static str>,
    pub last_was_kill: bool,
    pub prefix_arg: PrefixArg,
    pub should_quit: bool,
    pub pending_exit: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorState {
    pub fn new() -> Self {
        let mut buffers = BufferManager::new();
        let scratch_id = buffers.ensure_scratch();

        let mut windows = WindowManager::new();
        windows.add(scratch_id);

        Self {
            buffers,
            windows,
            minibuffer: Minibuffer::new(),
            kill_ring: KillRing::default(),
            keymap: default_keymap(),
            key_resolver: KeyResolver::new(),
            command_registry: build_default_registry(),
            message: None,
            last_command: None,
            last_was_kill: false,
            prefix_arg: PrefixArg::None,
            should_quit: false,
            pending_exit: false,
        }
    }

    pub fn current_buffer(&self) -> Option<&Buffer> {
        self.windows
            .current_buffer_id()
            .and_then(|id| self.buffers.get(id))
    }

    pub fn current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        self.windows
            .current_buffer_id()
            .and_then(|id| self.buffers.get_mut(id))
    }

    pub fn current_window(&self) -> Option<&Window> {
        self.windows.current()
    }

    pub fn current_window_mut(&mut self) -> Option<&mut Window> {
        self.windows.current_mut()
    }

    pub fn open_file(&mut self, path: PathBuf) -> std::io::Result<BufferId> {
        let existing_id = self.buffers.iter().find_map(|b| {
            if b.file_path.as_ref() == Some(&path) {
                Some(b.id)
            } else {
                None
            }
        });

        if let Some(id) = existing_id {
            self.buffers.set_current(id);
            self.windows.set_current_buffer(id);
            return Ok(id);
        }

        let buffer = Buffer::from_file(path)?;
        let id = self.buffers.add(buffer);
        self.buffers.set_current(id);
        self.windows.set_current_buffer(id);
        Ok(id)
    }

    pub fn switch_buffer(&mut self, name: &str) {
        if let Some(id) = self.buffers.find_by_name(name) {
            self.buffers.set_current(id);
            self.windows.set_current_buffer(id);
        } else {
            let buffer = Buffer::new(name);
            let id = self.buffers.add(buffer);
            self.buffers.set_current(id);
            self.windows.set_current_buffer(id);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.pending_exit {
            self.handle_exit_confirmation(key);
            return;
        }

        if self.minibuffer.is_active() {
            self.handle_minibuffer_key(key);
            return;
        }

        self.message = None;

        let resolution = self.key_resolver.resolve(key, &self.keymap);

        match resolution {
            KeyResolution::Complete(command_name) => {
                self.execute_command(command_name);
            }
            KeyResolution::Prefix(display) => {
                self.message = Some(display);
            }
            KeyResolution::SelfInsert(c) => {
                if let Err(e) = crate::commands::editing::self_insert(self, c) {
                    self.message = Some(format!("{}", e));
                }
                self.post_command("self-insert-command");
            }
            KeyResolution::Unbound(keys) => {
                let key_str: String = keys
                    .iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                self.message = Some(format!("{} is undefined", key_str));
            }
        }
    }

    fn execute_command(&mut self, name: &'static str) {
        let ctx = CommandContext {
            prefix_arg: std::mem::take(&mut self.prefix_arg),
            last_command: self.last_command,
        };

        let result = if let Some(cmd) = self.command_registry.get(name) {
            let exec = cmd.execute;
            exec(self, &ctx)
        } else {
            Err(crate::commands::registry::CommandError::NotFound(name.to_string()))
        };

        if let Err(e) = result {
            if !matches!(e, crate::commands::registry::CommandError::Cancelled) {
                self.message = Some(format!("{}", e));
            }
        }

        self.post_command(name);
    }

    fn post_command(&mut self, command_name: &'static str) {
        if let Some(cmd) = self.command_registry.get(command_name) {
            if !cmd.is_kill {
                self.last_was_kill = false;
                self.kill_ring.set_last_was_kill(false);
            }

            if !cmd.preserves_mark {
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.cursors.deactivate_all_marks();
                }
            }
        }

        self.last_command = Some(command_name);

        if let Some(buffer) = self.current_buffer_mut() {
            buffer.add_undo_boundary();
        }

        self.ensure_cursor_visible();
    }

    fn ensure_cursor_visible(&mut self) {
        use crate::core::rope_ext::RopeExt;

        let cursor_line = if let Some(buffer) = self.current_buffer() {
            buffer.text.char_to_position(buffer.cursors.primary.position).line
        } else {
            return;
        };

        if let Some(window) = self.current_window_mut() {
            let visible_start = window.scroll_line;
            let visible_end = window.scroll_line + (window.height as usize).saturating_sub(1);

            if cursor_line < visible_start {
                window.scroll_line = cursor_line;
            } else if cursor_line >= visible_end {
                window.scroll_line = cursor_line.saturating_sub((window.height as usize).saturating_sub(2));
            }
        }
    }

    fn handle_minibuffer_key(&mut self, key: KeyEvent) {
        use crate::keybinding::key::{Key, Modifiers};

        match (key.key, key.modifiers) {
            (Key::Enter, Modifiers::NONE) => {
                if let Some((content, callback)) = self.minibuffer.submit() {
                    self.handle_minibuffer_callback(callback, content);
                }
            }
            (Key::Char('g'), Modifiers::CTRL) | (Key::Escape, _) => {
                self.minibuffer.clear();
                self.message = Some("Quit".to_string());
            }
            (Key::Backspace, Modifiers::NONE) => {
                self.minibuffer.delete_backward();
            }
            (Key::Delete, Modifiers::NONE) | (Key::Char('d'), Modifiers::CTRL) => {
                self.minibuffer.delete_forward();
            }
            (Key::Char('f'), Modifiers::CTRL) | (Key::Right, Modifiers::NONE) => {
                self.minibuffer.move_forward();
            }
            (Key::Char('b'), Modifiers::CTRL) | (Key::Left, Modifiers::NONE) => {
                self.minibuffer.move_backward();
            }
            (Key::Char('a'), Modifiers::CTRL) | (Key::Home, Modifiers::NONE) => {
                self.minibuffer.move_to_start();
            }
            (Key::Char('e'), Modifiers::CTRL) | (Key::End, Modifiers::NONE) => {
                self.minibuffer.move_to_end();
            }
            (Key::Char('p'), Modifiers::CTRL) | (Key::Up, Modifiers::NONE) => {
                self.minibuffer.history_prev();
            }
            (Key::Char('n'), Modifiers::CTRL) | (Key::Down, Modifiers::NONE) => {
                self.minibuffer.history_next();
            }
            (Key::Char(c), Modifiers::NONE) => {
                self.minibuffer.insert_char(c);
            }
            (Key::Char(c), Modifiers::SHIFT) => {
                self.minibuffer.insert_char(c);
            }
            _ => {}
        }
    }

    fn handle_minibuffer_callback(&mut self, callback: &str, content: String) {
        match callback {
            "find-file-complete" => {
                let path = PathBuf::from(&content);
                match self.open_file(path) {
                    Ok(_) => {
                        self.message = Some(format!("Opened {}", content));
                    }
                    Err(e) => {
                        self.message = Some(format!("Error opening file: {}", e));
                    }
                }
            }
            "save-buffer-complete" | "write-file-complete" => {
                let path = PathBuf::from(&content);
                if let Some(buffer) = self.current_buffer_mut() {
                    match buffer.save_as(path) {
                        Ok(()) => {
                            self.message = Some(format!("Wrote {}", buffer.name));
                        }
                        Err(e) => {
                            self.message = Some(format!("Error saving: {}", e));
                        }
                    }
                }
            }
            "switch-to-buffer-complete" => {
                self.switch_buffer(&content);
            }
            "kill-buffer-complete" => {
                if let Some(id) = self.buffers.find_by_name(&content) {
                    let was_modified = self.buffers.get(id).map(|b| b.modified).unwrap_or(false);
                    if was_modified {
                        self.message = Some("Buffer has unsaved changes".to_string());
                    } else {
                        self.buffers.kill(id);
                        if self.buffers.is_empty() {
                            self.buffers.ensure_scratch();
                        }
                        if let Some(new_id) = self.buffers.current_id() {
                            self.windows.set_current_buffer(new_id);
                        }
                    }
                }
            }
            "execute-extended-command" => {
                if self.command_registry.get(&content).is_some() {
                    let name: &'static str = Box::leak(content.into_boxed_str());
                    self.execute_command(name);
                } else {
                    self.message = Some(format!("Unknown command: {}", content));
                }
            }
            _ => {}
        }
    }

    fn handle_exit_confirmation(&mut self, key: KeyEvent) {
        use crate::keybinding::key::Key;

        match key.key {
            Key::Char('y') | Key::Char('Y') => {
                self.should_quit = true;
            }
            Key::Char('n') | Key::Char('N') => {
                self.pending_exit = false;
                self.message = Some("Exit cancelled".to_string());
            }
            _ => {}
        }
    }

    pub fn start_minibuffer_prompt(&mut self, prompt: &str, callback: &'static str) {
        self.minibuffer.start_prompt(prompt, callback);
    }

    pub fn set_dimensions(&mut self, width: u16, height: u16) {
        self.windows.set_dimensions(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinding::key::Key;

    #[test]
    fn test_editor_state_new() {
        let state = EditorState::new();
        assert!(state.current_buffer().is_some());
        assert_eq!(state.current_buffer().unwrap().name, "*scratch*");
    }

    #[test]
    fn test_self_insert() {
        let mut state = EditorState::new();
        state.handle_key(KeyEvent::char('a'));
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "a");
    }

    #[test]
    fn test_command_execution() {
        let mut state = EditorState::new();
        state.handle_key(KeyEvent::char('h'));
        state.handle_key(KeyEvent::char('e'));
        state.handle_key(KeyEvent::char('l'));
        state.handle_key(KeyEvent::char('l'));
        state.handle_key(KeyEvent::char('o'));

        assert_eq!(state.current_buffer().unwrap().text.to_string(), "hello");

        state.handle_key(KeyEvent::ctrl('a'));
        assert_eq!(
            state.current_buffer().unwrap().cursors.primary.position.0,
            0
        );
    }
}
