use crate::core::position::CharOffset;
use crate::core::rope_ext::{find_word_boundary_backward, find_word_boundary_forward, RopeExt};
use crate::state::EditorState;

use super::registry::{Command, CommandContext, CommandError, CommandResult};

pub fn kill_line(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    let (start, end) = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        let pos = window.cursors.primary.position;
        let position = buffer.text.char_to_position(pos);
        let line_end = buffer.text.line_end_char(position.line);

        if pos == line_end {
            let next_char = CharOffset(pos.0 + 1);
            if next_char.0 <= buffer.len_chars() {
                (pos, next_char)
            } else {
                return Ok(());
            }
        } else {
            let mut end = line_end;
            for _ in 1..count {
                let next_line = buffer.text.char_to_position(end).line + 1;
                if next_line < buffer.text.total_lines() {
                    end = CharOffset(buffer.text.line_end_char(next_line).0 + 1);
                }
            }
            (pos, end)
        }
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_region(cursors, start, end)
    } else {
        String::new()
    };

    if !killed.is_empty() {
        state.kill_ring.push(killed, state.last_was_kill);
    }

    state.last_was_kill = true;
    Ok(())
}

pub fn kill_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    let (start, end) = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        let start = window.cursors.primary.position;
        let mut end = start;
        for _ in 0..count {
            end = find_word_boundary_forward(&buffer.text, end);
        }
        (start, end)
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_region(cursors, start, end)
    } else {
        String::new()
    };

    if !killed.is_empty() {
        state.kill_ring.push(killed, state.last_was_kill);
    }

    state.last_was_kill = true;
    Ok(())
}

pub fn backward_kill_word(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    let count = ctx.repeat_count();
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    let (start, end) = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        let end = window.cursors.primary.position;
        let mut start = end;
        for _ in 0..count {
            start = find_word_boundary_backward(&buffer.text, start);
        }
        (start, end)
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_region(cursors, start, end)
    } else {
        String::new()
    };

    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            if cursor.position >= end {
                cursor.position = start;
            }
        }
    }

    if !killed.is_empty() {
        state.kill_ring.push_prepend(killed);
    }

    state.last_was_kill = true;
    Ok(())
}

pub fn kill_region(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    let region = state.windows.current().and_then(|w| w.cursors.primary.region());
    let (start, end) = match region {
        Some((s, e)) => (s, e),
        None => {
            state.message = Some("The mark is not set now, so there is no region".to_string());
            return Err(CommandError::NoMark);
        }
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_region(cursors, start, end)
    } else {
        String::new()
    };

    if let Some(window) = state.windows.current_mut() {
        window.cursors.primary.deactivate_mark();
    }

    if !killed.is_empty() {
        state.kill_ring.push(killed, false);
    }

    state.last_was_kill = true;
    Ok(())
}

pub fn copy_region_as_kill(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let region = state.windows.current().and_then(|w| w.cursors.primary.region());
    let (start, end) = match region {
        Some((s, e)) => (s, e),
        None => {
            state.message = Some("The mark is not set now, so there is no region".to_string());
            return Err(CommandError::NoMark);
        }
    };

    let copied = state.buffers.get(buffer_id).map(|b| b.slice(start, end)).unwrap_or_default();

    if let Some(window) = state.windows.current_mut() {
        window.cursors.primary.deactivate_mark();
    }

    if !copied.is_empty() {
        state.kill_ring.push(copied, false);
    }
    state.message = Some("Region saved".to_string());

    Ok(())
}

pub fn yank(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    if let Some(text) = state.kill_ring.yank().map(|s| s.to_string()) {
        let start = state.windows.current().unwrap().cursors.primary.position;
        let cursors = &mut state.windows.current_mut().unwrap().cursors;
        if let Some(buffer) = state.buffers.get_mut(buffer_id) {
            buffer.insert_string(cursors, &text);
        }
        if let Some(window) = state.windows.current_mut() {
            window.cursors.primary.set_mark(start);
        }
        state.kill_ring.reset_yank_pointer();
    } else {
        state.message = Some("Kill ring is empty".to_string());
    }

    state.last_was_kill = false;
    Ok(())
}

pub fn yank_pop(state: &mut EditorState, ctx: &CommandContext) -> CommandResult {
    if ctx.last_command != Some("yank") && ctx.last_command != Some("yank-pop") {
        state.message = Some("Previous command was not a yank".to_string());
        return Ok(());
    }

    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let read_only = state.buffers.get(buffer_id).map(|b| b.read_only).unwrap_or(false);
    if read_only {
        return Err(CommandError::ReadOnly);
    }

    let start_pos = {
        let window = state.windows.current().unwrap();
        if let Some((mark, point)) = window.cursors.primary.mark.zip(Some(window.cursors.primary.position)) {
            let (start, end) = if mark < point { (mark, point) } else { (point, mark) };
            
            let cursors = &mut state.windows.current_mut().unwrap().cursors;
            if let Some(buffer) = state.buffers.get_mut(buffer_id) {
                buffer.delete_region(cursors, start, end);
            }
            if let Some(window) = state.windows.current_mut() {
                window.cursors.primary.position = start;
            }
            Some(start)
        } else {
            None
        }
    };

    if let Some(start) = start_pos {
        if let Some(text) = state.kill_ring.yank_pop().map(|s| s.to_string()) {
            let cursors = &mut state.windows.current_mut().unwrap().cursors;
            if let Some(buffer) = state.buffers.get_mut(buffer_id) {
                buffer.insert_string(cursors, &text);
            }
            if let Some(window) = state.windows.current_mut() {
                window.cursors.primary.set_mark(start);
            }
        }
    }

    Ok(())
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::kill("kill-line", kill_line),
        Command::kill("kill-word", kill_word),
        Command::kill("backward-kill-word", backward_kill_word),
        Command::kill("kill-region", kill_region),
        Command::new("copy-region-as-kill", copy_region_as_kill),
        Command::new("yank", yank),
        Command::new("yank-pop", yank_pop),
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
    fn test_kill_line() {
        let mut state = make_state("hello world\n");
        let ctx = CommandContext::new();

        kill_line(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "\n");
        assert_eq!(state.kill_ring.yank(), Some("hello world"));
    }

    #[test]
    fn test_kill_word() {
        let mut state = make_state("hello world");
        let ctx = CommandContext::new();

        kill_word(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), " world");
        assert_eq!(state.kill_ring.yank(), Some("hello"));
    }

    #[test]
    fn test_yank() {
        let mut state = make_state("");
        state.kill_ring.push("hello".to_string(), false);
        let ctx = CommandContext::new();

        yank(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "hello");
    }

    #[test]
    fn test_kill_region() {
        let mut state = make_state("hello world");
        state.windows.current_mut().unwrap().cursors.primary.position = CharOffset(6);
        state.windows.current_mut().unwrap().cursors.primary.set_mark(CharOffset(0));

        let ctx = CommandContext::new();
        kill_region(&mut state, &ctx).unwrap();

        assert_eq!(state.current_buffer().unwrap().text.to_string(), "world");
        assert_eq!(state.kill_ring.yank(), Some("hello "));
    }
}
