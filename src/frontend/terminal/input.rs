use crossterm::event::{Event, KeyEventKind, MouseButton, MouseEventKind as CrossMouseKind};

use crate::frontend::traits::{FrontendEvent, MouseEvent, MouseEventKind};
use crate::keybinding::KeyEvent;

pub fn convert_event(event: Event) -> Option<FrontendEvent> {
    match event {
        Event::Key(key_event) => {
            if key_event.kind == KeyEventKind::Press || key_event.kind == KeyEventKind::Repeat {
                Some(FrontendEvent::Key(KeyEvent::from(key_event)))
            } else {
                None
            }
        }
        Event::Resize(width, height) => Some(FrontendEvent::Resize(width, height)),
        Event::Mouse(mouse_event) => {
            let kind = match mouse_event.kind {
                CrossMouseKind::Down(MouseButton::Left) => Some(MouseEventKind::Down),
                CrossMouseKind::Up(MouseButton::Left) => Some(MouseEventKind::Up),
                CrossMouseKind::Drag(MouseButton::Left) => Some(MouseEventKind::Drag),
                CrossMouseKind::ScrollUp => Some(MouseEventKind::ScrollUp),
                CrossMouseKind::ScrollDown => Some(MouseEventKind::ScrollDown),
                _ => None,
            };

            kind.map(|k| {
                FrontendEvent::Mouse(MouseEvent {
                    kind: k,
                    column: mouse_event.column,
                    row: mouse_event.row,
                })
            })
        }
        Event::FocusGained => Some(FrontendEvent::Focus(true)),
        Event::FocusLost => Some(FrontendEvent::Focus(false)),
        _ => None,
    }
}
