use std::path::PathBuf;
use thiserror::Error;

use crate::keybinding::KeyEvent;
use crate::state::EditorState;

#[derive(Debug, Error)]
pub enum FrontendError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("GUI error: {0}")]
    Gui(String),

    #[error("Rendering error: {0}")]
    Render(String),
}

#[derive(Debug, Clone, Default)]
pub struct FrontendCapabilities {
    pub images: bool,
    pub true_color: bool,
    pub clipboard: bool,
    pub variable_width_fonts: bool,
}

pub trait Frontend {
    fn init(&mut self) -> Result<(), FrontendError>;

    fn shutdown(&mut self) -> Result<(), FrontendError>;

    fn size(&self) -> (u16, u16);

    fn run(self, state: EditorState) -> Result<(), FrontendError>;

    fn render(&mut self, state: &EditorState) -> Result<(), FrontendError>;

    fn bell(&mut self);

    fn capabilities(&self) -> FrontendCapabilities {
        FrontendCapabilities::default()
    }

    fn pixel_size(&self) -> Option<(u32, u32)> {
        None
    }

    fn scale_factor(&self) -> f32 {
        1.0
    }

    fn set_title(&mut self, _title: &str) {}
}

#[derive(Debug, Clone)]
pub enum FrontendEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),
    Focus(bool),
    FileDrop(Vec<PathBuf>),
    ScaleChange(f32),
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
    Move,
}
