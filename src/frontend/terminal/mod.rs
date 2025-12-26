mod input;
mod render;

use std::io::{self, Stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor,
    event,
    execute,
    terminal::{self, ClearType},
};

use crate::state::EditorState;

use super::traits::{Frontend, FrontendError, FrontendEvent};

pub struct TerminalFrontend {
    stdout: Stdout,
    width: u16,
    height: u16,
}

impl TerminalFrontend {
    pub fn new() -> Self {
        let (width, height) = terminal::size().unwrap_or((80, 24));
        Self {
            stdout: io::stdout(),
            width,
            height,
        }
    }

    fn poll_event(&mut self, timeout: Duration) -> Option<FrontendEvent> {
        if event::poll(timeout).ok()? {
            let event = event::read().ok()?;
            input::convert_event(event)
        } else {
            None
        }
    }
}

impl Default for TerminalFrontend {
    fn default() -> Self {
        Self::new()
    }
}

impl Frontend for TerminalFrontend {
    fn init(&mut self) -> Result<(), FrontendError> {
        terminal::enable_raw_mode()?;
        execute!(
            self.stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(ClearType::All)
        )?;
        let (width, height) = terminal::size()?;
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), FrontendError> {
        execute!(
            self.stdout,
            cursor::Show,
            terminal::LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn run(mut self, mut state: EditorState) -> Result<(), FrontendError> {
        loop {
            self.render(&state)?;

            if state.should_quit {
                break;
            }

            if let Some(event) = self.poll_event(Duration::from_millis(100)) {
                match event {
                    FrontendEvent::Key(key) => {
                        state.handle_key(key);
                    }
                    FrontendEvent::Resize(width, height) => {
                        state.set_dimensions(width, height);
                        self.width = width;
                        self.height = height;
                    }
                    FrontendEvent::Mouse(_) => {}
                    FrontendEvent::Focus(_) => {}
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, state: &EditorState) -> Result<(), FrontendError> {
        render::render(state, &mut self.stdout, self.width, self.height)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn bell(&mut self) {
        let _ = execute!(self.stdout, crossterm::terminal::SetTitle("\x07"));
    }
}

impl Drop for TerminalFrontend {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
