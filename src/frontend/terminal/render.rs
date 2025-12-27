use std::io::Stdout;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};

use crate::core::rope_ext::RopeExt;
use crate::state::EditorState;

pub fn render(
    state: &EditorState,
    stdout: &mut Stdout,
    width: u16,
    height: u16,
) -> std::io::Result<()> {
    for window in state.windows.iter() {
        render_window(state, stdout, window)?;
    }

    render_modeline(state, stdout, width, height)?;
    render_minibuffer(state, stdout, width, height)?;

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

                let mut in_any_region = false;
                for cursor in window.cursors.all_cursors() {
                    if let Some((start, end)) = cursor.region() {
                        if char_offset >= start.0 && char_offset < end.0 {
                            in_any_region = true;
                            break;
                        }
                    }
                }

                let is_cursor_pos = window
                    .cursors
                    .all_cursors()
                    .any(|c| c.position.0 == char_offset);
                let is_primary_cursor = window.cursors.primary.position.0 == char_offset;

                if is_primary_cursor {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::Black),
                        SetForegroundColor(Color::White)
                    )?;
                } else if in_any_region {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::Blue),
                        SetForegroundColor(Color::White)
                    )?;
                } else if is_cursor_pos {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::DarkGrey),
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

                if is_primary_cursor || in_any_region || is_cursor_pos {
                    queue!(stdout, ResetColor)?;
                }
            }

            let line_char_count = line_str.chars().count();
            let printed_len = line_str
                .chars()
                .map(|c| if c == '\t' { 4 } else { 1 })
                .sum::<usize>();

            let line_ends_with_newline = line_str.ends_with('\n');
            let cursor_at_eol = line_start_char + line_char_count;
            let check_eol_cursor = !line_ends_with_newline;
            let is_primary_at_eol =
                check_eol_cursor && window.cursors.primary.position.0 == cursor_at_eol;
            let is_any_cursor_at_eol = check_eol_cursor
                && window
                    .cursors
                    .all_cursors()
                    .any(|c| c.position.0 == cursor_at_eol);

            let mut first_pad = true;
            for _ in printed_len..window.width as usize {
                if first_pad && is_primary_at_eol {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::Black),
                        SetForegroundColor(Color::White),
                        Print(' '),
                        ResetColor
                    )?;
                } else if first_pad && is_any_cursor_at_eol {
                    queue!(
                        stdout,
                        SetBackgroundColor(Color::DarkGrey),
                        SetForegroundColor(Color::White),
                        Print(' '),
                        ResetColor
                    )?;
                } else {
                    queue!(stdout, Print(' '))?;
                }
                first_pad = false;
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
    let window = state.current_window();
    let buffer_name = buffer.map(|b| b.name.as_str()).unwrap_or("[No buffer]");
    let modified = buffer
        .map(|b| if b.modified { "**" } else { "--" })
        .unwrap_or("--");
    let readonly = buffer
        .map(|b| if b.read_only { "%%" } else { "--" })
        .unwrap_or("--");

    let mark_indicator = window
        .map(|w| {
            if w.cursors.primary.mark_active {
                " Mark"
            } else {
                ""
            }
        })
        .unwrap_or("");

    let cursor_indicator = window
        .map(|w| {
            if w.cursors.count() > 1 {
                format!(" [{}c]", w.cursors.count())
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    let (line, col) = match (buffer, window) {
        (Some(b), Some(w)) => {
            let pos = b.text.char_to_position(w.cursors.primary.position);
            (pos.line + 1, pos.column + 1)
        }
        _ => (1, 1),
    };

    let left = format!(
        "-{}:{}- {}{}{} ",
        modified, readonly, buffer_name, mark_indicator, cursor_indicator
    );
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

    let cursor_pos = if state.minibuffer.is_active() {
        Some(state.minibuffer.cursor_screen_pos())
    } else {
        None
    };

    for (i, ch) in content.chars().take(width as usize).enumerate() {
        if cursor_pos == Some(i) {
            queue!(
                stdout,
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::White),
                Print(ch),
                ResetColor
            )?;
        } else {
            queue!(stdout, Print(ch))?;
        }
    }

    let content_len = content.chars().take(width as usize).count();

    for i in content_len..width as usize {
        if cursor_pos == Some(i) {
            queue!(
                stdout,
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::White),
                Print(' '),
                ResetColor
            )?;
        } else {
            queue!(stdout, Print(' '))?;
        }
    }

    Ok(())
}
