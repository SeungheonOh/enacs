use thiserror::Error;

use crate::keybinding::KeyEvent;
use crate::state::EditorState;

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terminal error: {0}")]
    Terminal(String),
}

pub trait Frontend {
    fn init(&mut self) -> Result<(), FrontendError>;

    fn shutdown(&mut self) -> Result<(), FrontendError>;

    fn size(&self) -> (u16, u16);

    fn run(self, state: EditorState) -> Result<(), FrontendError>;

    fn render(&mut self, state: &EditorState) -> Result<(), FrontendError>;

    fn bell(&mut self);
}

#[derive(Debug, Clone)]
pub enum FrontendEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),
    Focus(bool),
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEventKind {
    Down,
    Up,
    Drag,
    ScrollUp,
    ScrollDown,
}
