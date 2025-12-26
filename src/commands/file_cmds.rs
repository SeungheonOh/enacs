use crate::state::EditorState;
use super::registry::{Command, CommandContext, CommandError, CommandResult};

pub fn find_file(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.start_minibuffer_prompt("Find file: ", "find-file-complete");
    Ok(())
}

pub fn save_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if let Some(buffer) = state.current_buffer_mut() {
        if buffer.file_path.is_none() {
            state.start_minibuffer_prompt("File to save in: ", "save-buffer-complete");
            return Ok(());
        }

        match buffer.save() {
            Ok(()) => {
                state.message = Some(format!("Wrote {}", buffer.name));
            }
            Err(e) => {
                state.message = Some(format!("Error saving: {}", e));
                return Err(CommandError::Io(e));
            }
        }
    }
    Ok(())
}

pub fn write_file(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.start_minibuffer_prompt("Write file: ", "write-file-complete");
    Ok(())
}

pub fn exit(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let has_modified = state.buffers.iter().any(|b| b.modified);
    if has_modified {
        state.message = Some("Modified buffers exist; really exit? (y or n) ".to_string());
        state.pending_exit = true;
    } else {
        state.should_quit = true;
    }
    Ok(())
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::new("find-file", find_file),
        Command::new("save-buffer", save_buffer),
        Command::new("write-file", write_file),
        Command::new("exit", exit),
    ]
}
