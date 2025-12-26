use crate::core::rope_ext::{find_word_boundary_backward, find_word_boundary_forward, RopeExt};
use crate::state::EditorState;

use super::registry::{Command, CommandContext, CommandResult};

pub fn forward_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_forward(count);
    }
    Ok(())
}

pub fn backward_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_backward(count);
    }
    Ok(())
}

pub fn next_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        for _ in 0..count {
            buffer.move_cursor_to_next_line();
        }
    }
    Ok(())
}

pub fn previous_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        for _ in 0..count {
            buffer.move_cursor_to_prev_line();
        }
    }
    Ok(())
}

pub fn move_beginning_of_line(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_to_line_start();
    }
    Ok(())
}

pub fn move_end_of_line(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_to_line_end();
    }
    Ok(())
}

pub fn beginning_of_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_to_buffer_start();
    }
    Ok(())
}

pub fn end_of_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.move_cursor_to_buffer_end();
    }
    Ok(())
}

pub fn forward_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        for _ in 0..count {
            for cursor in buffer.cursors.all_cursors_mut() {
                cursor.position = find_word_boundary_forward(&buffer.text, cursor.position);
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn backward_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        for _ in 0..count {
            for cursor in buffer.cursors.all_cursors_mut() {
                cursor.position = find_word_boundary_backward(&buffer.text, cursor.position);
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn scroll_up_command(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let (_lines_to_scroll, target_line) = {
        if let Some(window) = state.current_window_mut() {
            let scroll = if ctx.prefix_arg.is_set() {
                count
            } else {
                (window.height.saturating_sub(2)) as usize
            };
            window.scroll_line = window.scroll_line.saturating_add(scroll);
            (scroll, window.scroll_line)
        } else {
            return Ok(());
        }
    };

    if let Some(buffer) = state.current_buffer_mut() {
        for cursor in buffer.cursors.all_cursors_mut() {
            let pos = buffer.text.char_to_position(cursor.position);
            if pos.line < target_line {
                let line_start = buffer.text.line_start_char(target_line);
                cursor.position = line_start;
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn scroll_down_command(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let visible_end = {
        if let Some(window) = state.current_window_mut() {
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

    if let Some(buffer) = state.current_buffer_mut() {
        for cursor in buffer.cursors.all_cursors_mut() {
            let pos = buffer.text.char_to_position(cursor.position);
            if pos.line > visible_end {
                let line_start = buffer.text.line_start_char(visible_end);
                cursor.position = line_start;
                cursor.goal_column = None;
            }
        }
    }
    Ok(())
}

pub fn recenter_top_bottom(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer() {
        let cursor_pos = buffer.cursors.primary.position;
        let cursor_line = buffer.text.char_to_position(cursor_pos).line;

        if let Some(window) = state.current_window_mut() {
            let half_height = (window.height as usize) / 2;
            window.scroll_line = cursor_line.saturating_sub(half_height);
        }
    }
    Ok(())
}

pub fn goto_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let line = if ctx.prefix_arg.is_set() {
        (ctx.count() - 1).max(0) as usize
    } else {
        return Ok(());
    };

    if let Some(buffer) = state.current_buffer_mut() {
        let max_line = buffer.text.total_lines().saturating_sub(1);
        let target_line = line.min(max_line);
        let line_start = buffer.text.line_start_char(target_line);

        for cursor in buffer.cursors.all_cursors_mut() {
            cursor.position = line_start;
            cursor.goal_column = None;
        }
    }
    Ok(())
}

fn ensure_mark_for_shift_select(state: &mut EditorState) {
    if let Some(buffer) = state.current_buffer_mut() {
        for cursor in buffer.cursors.all_cursors_mut() {
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
    use crate::core::{Buffer, CharOffset};

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
        assert_eq!(state.current_buffer().unwrap().cursors.primary.position, CharOffset(1));
    }

    #[test]
    fn test_line_movement() {
        let mut state = make_state("hello\nworld\n");
        let ctx = CommandContext::new();

        next_line(&mut state, &ctx).unwrap();
        let pos = state.current_buffer().unwrap().cursors.primary.position;
        assert!(pos.0 >= 6);
    }

    #[test]
    fn test_word_movement() {
        let mut state = make_state("hello world foo");
        let ctx = CommandContext::new();

        forward_word(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().cursors.primary.position, CharOffset(5));

        forward_word(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().cursors.primary.position, CharOffset(11));
    }
}
