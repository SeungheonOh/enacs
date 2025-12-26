use crate::core::mark::Mark;
use crate::core::position::CharOffset;
use crate::state::EditorState;

use super::registry::{Command, CommandContext, CommandError, CommandResult};

pub fn self_insert(state: &mut EditorState, c: char) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }
        buffer.insert_char(c);
    }
    Ok(())
}

pub fn delete_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }
        for _ in 0..count {
            buffer.delete_char_forward();
        }
    }
    Ok(())
}

pub fn delete_backward_char(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }
        for _ in 0..count {
            buffer.delete_char_backward();
        }
    }
    Ok(())
}

pub fn newline(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }
        for _ in 0..count {
            buffer.insert_char('\n');
        }
    }
    Ok(())
}

pub fn open_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }
        let original_pos = buffer.cursors.primary.position;
        for _ in 0..count {
            buffer.insert_char('\n');
        }
        for cursor in buffer.cursors.all_cursors_mut() {
            cursor.position = original_pos;
        }
    }
    Ok(())
}

pub fn transpose_chars(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.read_only {
            return Err(CommandError::ReadOnly);
        }

        let len = buffer.len_chars();
        let pos = buffer.cursors.primary.position.0;

        if len < 2 {
            return Ok(());
        }

        let (first, second) = if pos == 0 {
            (0, 1)
        } else if pos >= len {
            (len - 2, len - 1)
        } else {
            (pos - 1, pos)
        };

        let c1 = buffer.text.char(first);
        let c2 = buffer.text.char(second);

        buffer.text.remove(first..second + 1);
        buffer.text.insert(first, &format!("{}{}", c2, c1));

        buffer.cursors.primary.position = CharOffset((second + 1).min(len));
        buffer.modified = true;
    }
    Ok(())
}

pub fn set_mark_command(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        for cursor in buffer.cursors.all_cursors_mut() {
            let pos = cursor.position;
            cursor.set_mark(pos);
        }

        let primary_pos = buffer.cursors.primary.position;
        buffer.mark_ring.push(Mark::new(primary_pos));

        state.message = Some("Mark set".to_string());
    }
    Ok(())
}

pub fn exchange_point_and_mark(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        for cursor in buffer.cursors.all_cursors_mut() {
            cursor.exchange_point_and_mark();
        }
    }
    Ok(())
}

pub fn mark_whole_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        let end = CharOffset(buffer.len_chars());

        buffer.mark_ring.push(Mark::new(buffer.cursors.primary.position));

        buffer.cursors.primary.position = end;
        buffer.cursors.primary.set_mark(CharOffset(0));
    }
    Ok(())
}

pub fn undo_command(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.undo() {
            state.message = Some("Undo!".to_string());
        } else {
            state.message = Some("No further undo information".to_string());
        }
    }
    Ok(())
}

pub fn keyboard_quit(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        buffer.cursors.deactivate_all_marks();
        buffer.cursors.remove_secondary_cursors();
    }

    state.minibuffer.clear();
    state.message = Some("Quit".to_string());
    Err(CommandError::Cancelled)
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::new("delete-char", delete_char),
        Command::new("delete-backward-char", delete_backward_char),
        Command::new("newline", newline),
        Command::new("open-line", open_line),
        Command::new("transpose-chars", transpose_chars),
        Command::mark("set-mark-command", set_mark_command),
        Command::mark("exchange-point-and-mark", exchange_point_and_mark),
        Command::mark("mark-whole-buffer", mark_whole_buffer),
        Command::new("undo", undo_command),
        Command::new("keyboard-quit", keyboard_quit),
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
    fn test_delete_char() {
        let mut state = make_state("hello");
        let ctx = CommandContext::new();

        delete_char(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "ello");
    }

    #[test]
    fn test_newline() {
        let mut state = make_state("hello");
        state.current_buffer_mut().unwrap().cursors.primary.position = CharOffset(5);
        let ctx = CommandContext::new();

        newline(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "hello\n");
    }

    #[test]
    fn test_transpose_chars() {
        let mut state = make_state("ab");
        state.current_buffer_mut().unwrap().cursors.primary.position = CharOffset(1);
        let ctx = CommandContext::new();

        transpose_chars(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "ba");
    }
}
