#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinibufferState {
    Inactive,
    Prompt,
    Reading,
}

#[derive(Debug)]
pub struct Minibuffer {
    pub state: MinibufferState,
    pub prompt: String,
    pub content: String,
    pub cursor_pos: usize,
    pub callback: Option<&'static str>,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl Default for Minibuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Minibuffer {
    pub fn new() -> Self {
        Self {
            state: MinibufferState::Inactive,
            prompt: String::new(),
            content: String::new(),
            cursor_pos: 0,
            callback: None,
            history: Vec::new(),
            history_index: None,
        }
    }

    pub fn start_prompt(&mut self, prompt: &str, callback: &'static str) {
        self.state = MinibufferState::Prompt;
        self.prompt = prompt.to_string();
        self.content.clear();
        self.cursor_pos = 0;
        self.callback = Some(callback);
        self.history_index = None;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos <= self.content.len() {
            self.content.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
        }
    }

    pub fn delete_backward(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.content.remove(self.cursor_pos);
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.content.remove(self.cursor_pos);
        }
    }

    pub fn move_forward(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_backward(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_to_end(&mut self) {
        self.cursor_pos = self.content.len();
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(0) => return,
            Some(i) => i - 1,
        };

        self.history_index = Some(new_index);
        self.content = self.history[new_index].clone();
        self.cursor_pos = self.content.len();
    }

    pub fn history_next(&mut self) {
        match self.history_index {
            None => (),
            Some(i) if i >= self.history.len() - 1 => {
                self.history_index = None;
                self.content.clear();
                self.cursor_pos = 0;
            }
            Some(i) => {
                self.history_index = Some(i + 1);
                self.content = self.history[i + 1].clone();
                self.cursor_pos = self.content.len();
            }
        }
    }

    pub fn submit(&mut self) -> Option<(String, &'static str)> {
        if self.state == MinibufferState::Inactive {
            return None;
        }

        let content = std::mem::take(&mut self.content);
        let callback = self.callback.take();

        if !content.is_empty() {
            self.history.push(content.clone());
        }

        self.clear();

        callback.map(|cb| (content, cb))
    }

    pub fn clear(&mut self) {
        self.state = MinibufferState::Inactive;
        self.prompt.clear();
        self.content.clear();
        self.cursor_pos = 0;
        self.callback = None;
        self.history_index = None;
    }

    pub fn is_active(&self) -> bool {
        self.state != MinibufferState::Inactive
    }

    pub fn display(&self) -> String {
        format!("{}{}", self.prompt, self.content)
    }

    pub fn cursor_screen_pos(&self) -> usize {
        self.prompt.len() + self.cursor_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minibuffer_input() {
        let mut mb = Minibuffer::new();
        mb.start_prompt("Test: ", "test-callback");

        mb.insert_char('h');
        mb.insert_char('e');
        mb.insert_char('l');
        mb.insert_char('l');
        mb.insert_char('o');

        assert_eq!(mb.content, "hello");
        assert_eq!(mb.cursor_pos, 5);
    }

    #[test]
    fn test_minibuffer_delete() {
        let mut mb = Minibuffer::new();
        mb.start_prompt("Test: ", "test-callback");
        mb.content = "hello".to_string();
        mb.cursor_pos = 5;

        mb.delete_backward();
        assert_eq!(mb.content, "hell");

        mb.cursor_pos = 0;
        mb.delete_forward();
        assert_eq!(mb.content, "ell");
    }

    #[test]
    fn test_minibuffer_submit() {
        let mut mb = Minibuffer::new();
        mb.start_prompt("Test: ", "test-callback");
        mb.content = "hello".to_string();

        let result = mb.submit();
        assert_eq!(result, Some(("hello".to_string(), "test-callback")));
        assert!(!mb.is_active());
    }
}
