use std::io::Stdout;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
};

use crate::core::rope_ext::RopeExt;
use crate::state::EditorState;

pub fn render(
    state: &EditorState,
    stdout: &mut Stdout,
    width: u16,
    height: u16,
) -> std::io::Result<()> {
    queue!(stdout, Hide)?;

    for window in state.windows.iter() {
        render_window(state, stdout, window)?;
    }

    render_modeline(state, stdout, width, height)?;
    render_minibuffer(state, stdout, width, height)?;

    if state.minibuffer.is_active() {
        let cursor_x = state.minibuffer.cursor_screen_pos() as u16;
        queue!(stdout, MoveTo(cursor_x.min(width - 1), height - 1))?;
    } else if let Some(window) = state.windows.current() {
        if let Some(buffer) = state.buffers.get(window.buffer_id) {
            let cursor_pos = buffer.cursors.primary.position;
            let pos = buffer.text.char_to_position(cursor_pos);

            let screen_line = pos.line.saturating_sub(window.scroll_line);
            let screen_col = pos.column.saturating_sub(window.scroll_column);

            let x = (window.x as usize + screen_col).min((window.x + window.width - 1) as usize);
            let y = (window.y as usize + screen_line).min((window.y + window.height - 1) as usize);

            queue!(stdout, MoveTo(x as u16, y as u16))?;
        }
    }

    queue!(stdout, Show)?;

    Ok(())
}

fn render_window(
    state: &EditorState,
    stdout: &mut Stdout,
    window: &crate::state::Window,
) -> std::io::Result<()> {
    let buffer = match state.buffers.get(window.buffer_id) {
        Some(b) => b,
        None => return Ok(()),
    };

    let region = buffer.cursors.primary.region();

    for row in 0..window.height {
        let line_idx = window.scroll_line + row as usize;
        let y = window.y + row;

        queue!(stdout, MoveTo(window.x, y))?;

        if line_idx < buffer.text.total_lines() {
            let line = buffer.text.line(line_idx);
            let line_str: String = line.chars().take(window.width as usize).collect();

            let line_start_char = buffer.text.line_start_char(line_idx).0;

            for (col, ch) in line_str.chars().enumerate() {
                if col >= window.width as usize {
                    break;
                }

                let char_offset = line_start_char + col;

                let in_region = region
                    .map(|(start, end)| char_offset >= start.0 && char_offset < end.0)
                    .unwrap_or(false);

                if in_region {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::Blue),
                        SetForegroundColor(Color::White)
                    )?;
                }

                if ch == '\n' {
                    queue!(stdout, Print(' '))?;
                } else if ch == '\t' {
                    queue!(stdout, Print("    "))?;
                } else {
                    queue!(stdout, Print(ch))?;
                }

                if in_region {
                    queue!(stdout, ResetColor)?;
                }
            }

            let printed_len = line_str
                .chars()
                .map(|c| if c == '\t' { 4 } else { 1 })
                .sum::<usize>();

            for _ in printed_len..window.width as usize {
                queue!(stdout, Print(' '))?;
            }
        } else {
            queue!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print('~'),
                ResetColor
            )?;
            for _ in 1..window.width {
                queue!(stdout, Print(' '))?;
            }
        }
    }

    Ok(())
}

fn render_modeline(
    state: &EditorState,
    stdout: &mut Stdout,
    width: u16,
    height: u16,
) -> std::io::Result<()> {
    let modeline_y = height - 2;

    queue!(
        stdout,
        MoveTo(0, modeline_y),
        SetBackgroundColor(Color::White),
        SetForegroundColor(Color::Black),
        SetAttribute(Attribute::Bold)
    )?;

    let buffer = state.current_buffer();
    let buffer_name = buffer.map(|b| b.name.as_str()).unwrap_or("[No buffer]");
    let modified = buffer.map(|b| if b.modified { "**" } else { "--" }).unwrap_or("--");
    let readonly = buffer.map(|b| if b.read_only { "%%" } else { "--" }).unwrap_or("--");

    let mark_indicator = buffer
        .map(|b| if b.cursors.primary.mark_active { " Mark" } else { "" })
        .unwrap_or("");

    let (line, col) = buffer
        .map(|b| {
            let pos = b.text.char_to_position(b.cursors.primary.position);
            (pos.line + 1, pos.column + 1)
        })
        .unwrap_or((1, 1));

    let left = format!("-{}:{}- {}{} ", modified, readonly, buffer_name, mark_indicator);
    let right = format!(" L{}:C{} ", line, col);

    let padding = (width as usize).saturating_sub(left.len() + right.len());

    queue!(stdout, Print(&left))?;
    for _ in 0..padding {
        queue!(stdout, Print('-'))?;
    }
    queue!(stdout, Print(&right))?;

    queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;

    Ok(())
}

fn render_minibuffer(
    state: &EditorState,
    stdout: &mut Stdout,
    width: u16,
    height: u16,
) -> std::io::Result<()> {
    let y = height - 1;

    queue!(stdout, MoveTo(0, y), ResetColor)?;

    let content = if state.minibuffer.is_active() {
        state.minibuffer.display()
    } else if let Some(ref msg) = state.message {
        msg.clone()
    } else if state.key_resolver.is_pending() {
        state.key_resolver.pending_display().to_string()
    } else {
        String::new()
    };

    let display: String = content.chars().take(width as usize).collect();
    queue!(stdout, Print(&display))?;

    for _ in display.len()..width as usize {
        queue!(stdout, Print(' '))?;
    }

    Ok(())
}
