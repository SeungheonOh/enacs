use crate::core::position::CharOffset;
use crate::core::rope_ext::{find_word_boundary_backward, find_word_boundary_forward, RopeExt};
use crate::state::EditorState;

use super::registry::{Command, CommandContext, CommandResult};

pub fn forward_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let max = state.current_buffer().map(|b| b.text.len_chars()).unwrap_or(0);
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            cursor.position = CharOffset((cursor.position.0 + count).min(max));
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn backward_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            cursor.position = CharOffset(cursor.position.0.saturating_sub(count));
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn next_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    for _ in 0..count {
        if let Some(window) = state.windows.current_mut() {
            let buffer = match state.buffers.get(buffer_id) {
                Some(b) => b,
                None => return Ok(()),
            };

            for cursor in window.cursors.all_cursors_mut() {
                let pos = buffer.text.char_to_position(cursor.position);
                let goal_col = cursor.goal_column.unwrap_or(pos.column);
                let total_lines = buffer.text.total_lines();

                if pos.line + 1 < total_lines {
                    let next_line = pos.line + 1;
                    let line_len = buffer.text.line_len_chars(next_line);
                    let new_col = goal_col.min(line_len);
                    let line_start = buffer.text.line_start_char(next_line);
                    cursor.position = CharOffset(line_start.0 + new_col);
                    cursor.goal_column = Some(goal_col);
                }
            }
        }
    }
    Ok(())
}

pub fn previous_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    for _ in 0..count {
        if let Some(window) = state.windows.current_mut() {
            let buffer = match state.buffers.get(buffer_id) {
                Some(b) => b,
                None => return Ok(()),
            };

            for cursor in window.cursors.all_cursors_mut() {
                let pos = buffer.text.char_to_position(cursor.position);
                let goal_col = cursor.goal_column.unwrap_or(pos.column);

                if pos.line > 0 {
                    let prev_line = pos.line - 1;
                    let line_len = buffer.text.line_len_chars(prev_line);
                    let new_col = goal_col.min(line_len);
                    let line_start = buffer.text.line_start_char(prev_line);
                    cursor.position = CharOffset(line_start.0 + new_col);
                    cursor.goal_column = Some(goal_col);
                }
            }
        }
    }
    Ok(())
}

pub fn move_beginning_of_line(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    if let Some(window) = state.windows.current_mut() {
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        for cursor in window.cursors.all_cursors_mut() {
            let pos = buffer.text.char_to_position(cursor.position);
            let new_pos = buffer.text.line_start_char(pos.line);
            cursor.position = new_pos;
            cursor.goal_column = Some(0);
        }
    }
    Ok(())
}

pub fn move_end_of_line(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    if let Some(window) = state.windows.current_mut() {
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        for cursor in window.cursors.all_cursors_mut() {
            let pos = buffer.text.char_to_position(cursor.position);
            let new_pos = buffer.text.line_end_char(pos.line);
            cursor.position = new_pos;
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn beginning_of_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            cursor.position = CharOffset(0);
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn end_of_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let end = state.current_buffer().map(|b| b.text.len_chars()).unwrap_or(0);
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            cursor.position = CharOffset(end);
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn forward_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    if let Some(window) = state.windows.current_mut() {
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        for cursor in window.cursors.all_cursors_mut() {
            for _ in 0..count {
                let new_pos = find_word_boundary_forward(&buffer.text, cursor.position);
                cursor.position = new_pos;
            }
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn backward_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    if let Some(window) = state.windows.current_mut() {
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        for cursor in window.cursors.all_cursors_mut() {
            for _ in 0..count {
                let new_pos = find_word_boundary_backward(&buffer.text, cursor.position);
                cursor.position = new_pos;
            }
            cursor.goal_column = None;
        }
    }
    Ok(())
}

pub fn scroll_up_command(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let target_line = {
        if let Some(window) = state.windows.current_mut() {
            let scroll = if ctx.prefix_arg.is_set() {
                count
            } else {
                (window.height.saturating_sub(2)) as usize
            };
            window.scroll_line = window.scroll_line.saturating_add(scroll);
            window.scroll_line
        } else {
            return Ok(());
        }
    };

    let new_pos = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let pos = buffer.text.char_to_position(window.cursors.primary.position);
        if pos.line < target_line {
            Some(buffer.text.line_start_char(target_line))
        } else {
            None
        }
    };

    if let Some(new_pos) = new_pos {
        if let Some(window) = state.windows.current_mut() {
            for cursor in window.cursors.all_cursors_mut() {
                cursor.position = new_pos;
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn scroll_down_command(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let visible_end = {
        if let Some(window) = state.windows.current_mut() {
            let lines_to_scroll = if ctx.prefix_arg.is_set() {
                count
            } else {
                (window.height.saturating_sub(2)) as usize
            };
            window.scroll_line = window.scroll_line.saturating_sub(lines_to_scroll);
            window.scroll_line + (window.height as usize).saturating_sub(2)
        } else {
            return Ok(());
        }
    };

    let new_pos = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let pos = buffer.text.char_to_position(window.cursors.primary.position);
        if pos.line > visible_end {
            Some(buffer.text.line_start_char(visible_end))
        } else {
            None
        }
    };

    if let Some(new_pos) = new_pos {
        if let Some(window) = state.windows.current_mut() {
            for cursor in window.cursors.all_cursors_mut() {
                cursor.position = new_pos;
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn recenter_top_bottom(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let cursor_line = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        buffer.text.char_to_position(window.cursors.primary.position).line
    };

    if let Some(window) = state.windows.current_mut() {
        let half_height = (window.height as usize) / 2;
        window.scroll_line = cursor_line.saturating_sub(half_height);
    }
    Ok(())
}

pub fn goto_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let line = if ctx.prefix_arg.is_set() {
        (ctx.count() - 1).max(0) as usize
    } else {
        return Ok(());
    };

    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    if let Some(window) = state.windows.current_mut() {
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };
        let max_line = buffer.text.total_lines().saturating_sub(1);
        let target_line = line.min(max_line);
        let new_pos = buffer.text.line_start_char(target_line);

        for cursor in window.cursors.all_cursors_mut() {
            cursor.position = new_pos;
            cursor.goal_column = None;
        }
    }
    Ok(())
}

fn ensure_mark_for_shift_select(state: &mut EditorState) {
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            if !cursor.mark_active {
                let pos = cursor.position;
                cursor.set_mark(pos);
            }
        }
    }
}

pub fn forward_char_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    forward_char(state, ctx)
}

pub fn backward_char_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    backward_char(state, ctx)
}

pub fn next_line_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    next_line(state, ctx)
}

pub fn previous_line_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    previous_line(state, ctx)
}

pub fn forward_word_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    forward_word(state, ctx)
}

pub fn backward_word_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    backward_word(state, ctx)
}

pub fn move_beginning_of_line_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    move_beginning_of_line(state, ctx)
}

pub fn move_end_of_line_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    move_end_of_line(state, ctx)
}

pub fn beginning_of_buffer_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    beginning_of_buffer(state, ctx)
}

pub fn end_of_buffer_shift(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    ensure_mark_for_shift_select(state);
    end_of_buffer(state, ctx)
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::motion("forward-char", forward_char),
        Command::motion("backward-char", backward_char),
        Command::motion("next-line", next_line),
        Command::motion("previous-line", previous_line),
        Command::motion("move-beginning-of-line", move_beginning_of_line),
        Command::motion("move-end-of-line", move_end_of_line),
        Command::motion("beginning-of-buffer", beginning_of_buffer),
        Command::motion("end-of-buffer", end_of_buffer),
        Command::motion("forward-word", forward_word),
        Command::motion("backward-word", backward_word),
        Command::motion("scroll-up-command", scroll_up_command),
        Command::motion("scroll-down-command", scroll_down_command),
        Command::motion("recenter-top-bottom", recenter_top_bottom),
        Command::motion("goto-line", goto_line),
        Command::mark("forward-char-shift", forward_char_shift),
        Command::mark("backward-char-shift", backward_char_shift),
        Command::mark("next-line-shift", next_line_shift),
        Command::mark("previous-line-shift", previous_line_shift),
        Command::mark("forward-word-shift", forward_word_shift),
        Command::mark("backward-word-shift", backward_word_shift),
        Command::mark("move-beginning-of-line-shift", move_beginning_of_line_shift),
        Command::mark("move-end-of-line-shift", move_end_of_line_shift),
        Command::mark("beginning-of-buffer-shift", beginning_of_buffer_shift),
        Command::mark("end-of-buffer-shift", end_of_buffer_shift),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Buffer;

    fn make_state(content: &str) -> EditorState {
        let mut state = EditorState::new();
        let buffer = Buffer::from_string("test", content);
        let id = state.buffers.add(buffer);
        state.buffers.set_current(id);
        state.windows.set_current_buffer(id);
        state
    }

    #[test]
    fn test_forward_char() {
        let mut state = make_state("hello");
        let ctx = CommandContext::new();

        forward_char(&mut state, &ctx).unwrap();
        assert_eq!(state.windows.current().unwrap().cursors.primary.position, CharOffset(1));
    }

    #[test]
    fn test_line_movement() {
        let mut state = make_state("hello\nworld\n");
        let ctx = CommandContext::new();

        next_line(&mut state, &ctx).unwrap();
        let pos = state.windows.current().unwrap().cursors.primary.position;
        assert!(pos.0 >= 6);
    }

    #[test]
    fn test_word_movement() {
        let mut state = make_state("hello world foo");
        let ctx = CommandContext::new();

        forward_word(&mut state, &ctx).unwrap();
        assert_eq!(state.windows.current().unwrap().cursors.primary.position, CharOffset(5));

        forward_word(&mut state, &ctx).unwrap();
        assert_eq!(state.windows.current().unwrap().cursors.primary.position, CharOffset(11));
    }
}
