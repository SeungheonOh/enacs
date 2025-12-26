use crate::core::cursor::CursorId;
use crate::core::position::CharOffset;
use crate::core::rope_ext::{find_word_boundary_backward, find_word_boundary_forward, RopeExt};
use crate::state::EditorState;

use super::registry::{Command, CommandContext, CommandError, CommandResult};

fn update_kill_rings(
    state: &mut EditorState,
    killed: Vec<(CursorId, String)>,
    prepend: bool,
) {
    let window = match state.windows.current_mut() {
        Some(w) => w,
        None => return,
    };

    for (cursor_id, text) in killed {
        if text.is_empty() {
            continue;
        }

        if let Some(cursor) = window.cursors.get_by_id_mut(cursor_id) {
            if prepend {
                cursor.kill_ring.push_prepend(text);
            } else {
                cursor.kill_ring.push(text, cursor.kill_ring.last_was_kill());
            }
        }
    }
}

fn set_all_last_was_kill(state: &mut EditorState, was_kill: bool) {
    if let Some(window) = state.windows.current_mut() {
        for cursor in window.cursors.all_cursors_mut() {
            cursor.kill_ring.set_last_was_kill(was_kill);
        }
    }
}

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

    let regions: Vec<(CursorId, CharOffset, CharOffset)> = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        window
            .cursors
            .all_cursors()
            .filter_map(|cursor| {
                let pos = cursor.position;
                let position = buffer.text.char_to_position(pos);
                let line_end = buffer.text.line_end_char(position.line);

                let region = if pos == line_end {
                    let next_char = CharOffset(pos.0 + 1);
                    if next_char.0 <= buffer.len_chars() {
                        Some((pos, next_char))
                    } else {
                        None
                    }
                } else {
                    let mut end = line_end;
                    for _ in 1..count {
                        let next_line = buffer.text.char_to_position(end).line + 1;
                        if next_line < buffer.text.total_lines() {
                            end = CharOffset(buffer.text.line_end_char(next_line).0 + 1);
                        }
                    }
                    Some((pos, end))
                };

                region.map(|(start, end)| (cursor.id, start, end))
            })
            .collect()
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_regions(cursors, regions)
    } else {
        Vec::new()
    };

    update_kill_rings(state, killed, false);
    set_all_last_was_kill(state, true);

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

    let regions: Vec<(CursorId, CharOffset, CharOffset)> = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        window
            .cursors
            .all_cursors()
            .map(|cursor| {
                let start = cursor.position;
                let mut end = start;
                for _ in 0..count {
                    end = find_word_boundary_forward(&buffer.text, end);
                }
                (cursor.id, start, end)
            })
            .collect()
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_regions(cursors, regions)
    } else {
        Vec::new()
    };

    update_kill_rings(state, killed, false);
    set_all_last_was_kill(state, true);

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

    let regions: Vec<(CursorId, CharOffset, CharOffset)> = {
        let window = state.windows.current().unwrap();
        let buffer = match state.buffers.get(buffer_id) {
            Some(b) => b,
            None => return Ok(()),
        };

        window
            .cursors
            .all_cursors()
            .map(|cursor| {
                let end = cursor.position;
                let mut start = end;
                for _ in 0..count {
                    start = find_word_boundary_backward(&buffer.text, start);
                }
                (cursor.id, start, end)
            })
            .collect()
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_regions(cursors, regions)
    } else {
        Vec::new()
    };

    update_kill_rings(state, killed, true);
    set_all_last_was_kill(state, true);

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

    let regions: Vec<(CursorId, CharOffset, CharOffset)> = {
        let window = state.windows.current().unwrap();

        window
            .cursors
            .all_cursors()
            .filter_map(|cursor| cursor.region().map(|(start, end)| (cursor.id, start, end)))
            .collect()
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    let killed = if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_regions(cursors, regions)
    } else {
        Vec::new()
    };

    update_kill_rings(state, killed, false);

    if let Some(window) = state.windows.current_mut() {
        window.cursors.deactivate_all_marks();
    }

    set_all_last_was_kill(state, true);

    Ok(())
}

pub fn copy_region_as_kill(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let buffer_id = match state.windows.current() {
        Some(w) => w.buffer_id,
        None => return Ok(()),
    };

    let window = state.windows.current().unwrap();
    let buffer = match state.buffers.get(buffer_id) {
        Some(b) => b,
        None => return Ok(()),
    };

    let copies: Vec<(CursorId, String)> = window
        .cursors
        .all_cursors()
        .filter_map(|cursor| {
            cursor.region().map(|(start, end)| {
                let text = buffer.slice(start, end);
                (cursor.id, text)
            })
        })
        .collect();

    if copies.is_empty() {
        return Ok(());
    }

    let window_mut = state.windows.current_mut().unwrap();

    for (cursor_id, text) in copies {
        if text.is_empty() {
            continue;
        }

        if let Some(cursor) = window_mut.cursors.get_by_id_mut(cursor_id) {
            cursor.kill_ring.push(text, false);
            cursor.deactivate_mark();
        }
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

    let texts: Vec<(CursorId, CharOffset, String)> = {
        let window = state.windows.current().unwrap();

        window
            .cursors
            .all_cursors()
            .filter_map(|cursor| {
                cursor
                    .kill_ring
                    .yank()
                    .map(|s| (cursor.id, cursor.position, s.to_string()))
            })
            .collect()
    };

    if texts.is_empty() {
        state.message = Some("Kill ring is empty".to_string());
        return Ok(());
    }

    let mark_positions: Vec<(CursorId, CharOffset)> = texts
        .iter()
        .map(|(id, pos, _)| (*id, *pos))
        .collect();

    let insert_ops: Vec<(CursorId, String)> = texts
        .into_iter()
        .map(|(id, _, text)| (id, text))
        .collect();

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.insert_at_cursors(cursors, insert_ops);
    }

    let window_mut = state.windows.current_mut().unwrap();
    for (cursor_id, mark_pos) in mark_positions {
        if let Some(cursor) = window_mut.cursors.get_by_id_mut(cursor_id) {
            cursor.set_mark(mark_pos);
            cursor.kill_ring.reset_yank_pointer();
        }
    }

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

    let delete_regions: Vec<(CursorId, CharOffset, CharOffset)> = {
        let window = state.windows.current().unwrap();

        window
            .cursors
            .all_cursors()
            .filter_map(|cursor| {
                cursor.mark.map(|mark| {
                    let (start, end) = if mark < cursor.position {
                        (mark, cursor.position)
                    } else {
                        (cursor.position, mark)
                    };
                    (cursor.id, start, end)
                })
            })
            .collect()
    };

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.delete_regions(cursors, delete_regions);
    }

    let insert_texts: Vec<(CursorId, CharOffset, String)> = {
        let window_mut = state.windows.current_mut().unwrap();

        window_mut
            .cursors
            .all_cursors_mut()
            .filter_map(|cursor| {
                cursor
                    .kill_ring
                    .yank_pop()
                    .map(|s| (cursor.id, cursor.position, s.to_string()))
            })
            .collect()
    };

    if insert_texts.is_empty() {
        return Ok(());
    }

    let mark_positions: Vec<(CursorId, CharOffset)> = insert_texts
        .iter()
        .map(|(id, pos, _)| (*id, *pos))
        .collect();

    let insert_ops: Vec<(CursorId, String)> = insert_texts
        .into_iter()
        .map(|(id, _, text)| (id, text))
        .collect();

    let cursors = &mut state.windows.current_mut().unwrap().cursors;
    if let Some(buffer) = state.buffers.get_mut(buffer_id) {
        buffer.insert_at_cursors(cursors, insert_ops);
    }

    let window_mut = state.windows.current_mut().unwrap();
    for (cursor_id, mark_pos) in mark_positions {
        if let Some(cursor) = window_mut.cursors.get_by_id_mut(cursor_id) {
            cursor.set_mark(mark_pos);
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
        assert_eq!(
            state
                .windows
                .current()
                .unwrap()
                .cursors
                .primary
                .kill_ring
                .yank(),
            Some("hello world")
        );
    }

    #[test]
    fn test_kill_word() {
        let mut state = make_state("hello world");
        let ctx = CommandContext::new();

        kill_word(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), " world");
        assert_eq!(
            state
                .windows
                .current()
                .unwrap()
                .cursors
                .primary
                .kill_ring
                .yank(),
            Some("hello")
        );
    }

    #[test]
    fn test_yank() {
        let mut state = make_state("");
        state
            .windows
            .current_mut()
            .unwrap()
            .cursors
            .primary
            .kill_ring
            .push("hello".to_string(), false);
        let ctx = CommandContext::new();

        yank(&mut state, &ctx).unwrap();
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "hello");
    }

    #[test]
    fn test_kill_region() {
        let mut state = make_state("hello world");
        state.windows.current_mut().unwrap().cursors.primary.position = CharOffset(6);
        state
            .windows
            .current_mut()
            .unwrap()
            .cursors
            .primary
            .set_mark(CharOffset(0));

        let ctx = CommandContext::new();
        kill_region(&mut state, &ctx).unwrap();

        assert_eq!(state.current_buffer().unwrap().text.to_string(), "world");
        assert_eq!(
            state
                .windows
                .current()
                .unwrap()
                .cursors
                .primary
                .kill_ring
                .yank(),
            Some("hello ")
        );
    }

    #[test]
    fn test_multi_cursor_kill_word() {
        let mut state = make_state("aaa bbb ccc");
        let window = state.windows.current_mut().unwrap();
        window.cursors.primary.position = CharOffset(0);
        window.cursors.add_cursor(CharOffset(4));
        window.cursors.add_cursor(CharOffset(8));

        let ctx = CommandContext::new();
        kill_word(&mut state, &ctx).unwrap();

        // "aaa bbb ccc" has 2 spaces, after killing 3 words we have 2 spaces left
        assert_eq!(state.current_buffer().unwrap().text.to_string(), "  ");

        let window = state.windows.current().unwrap();
        assert_eq!(window.cursors.primary.kill_ring.yank(), Some("aaa"));

        let secondary_rings: Vec<_> = window
            .cursors
            .secondary
            .iter()
            .map(|c| c.kill_ring.yank())
            .collect();
        assert_eq!(secondary_rings, vec![Some("bbb"), Some("ccc")]);
    }

    #[test]
    fn test_multi_cursor_yank() {
        let mut state = make_state("X Y Z");
        let window = state.windows.current_mut().unwrap();
        window.cursors.primary.position = CharOffset(0);
        window.cursors.primary.kill_ring.push("AAA".to_string(), false);
        window.cursors.add_cursor(CharOffset(2));
        window.cursors.add_cursor(CharOffset(4));

        if let Some(c) = window.cursors.secondary.get_mut(0) {
            c.kill_ring.push("BBB".to_string(), false);
        }
        if let Some(c) = window.cursors.secondary.get_mut(1) {
            c.kill_ring.push("CCC".to_string(), false);
        }

        let ctx = CommandContext::new();
        yank(&mut state, &ctx).unwrap();

        assert_eq!(
            state.current_buffer().unwrap().text.to_string(),
            "AAAX BBBY CCCZ"
        );
    }
}
