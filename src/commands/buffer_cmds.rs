use crate::state::EditorState;
use super::registry::{Command, CommandContext, CommandResult};

pub fn switch_to_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.start_minibuffer_prompt("Switch to buffer: ", "switch-to-buffer-complete");
    Ok(())
}

pub fn kill_buffer(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.start_minibuffer_prompt("Kill buffer: ", "kill-buffer-complete");
    Ok(())
}

pub fn list_buffers(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    let mut msg = String::from("Buffers:\n");
    for buffer in state.buffers.iter() {
        let modified = if buffer.modified { "*" } else { " " };
        let readonly = if buffer.read_only { "%" } else { " " };
        msg.push_str(&format!("  {}{} {}\n", modified, readonly, buffer.name));
    }
    state.message = Some(msg);
    Ok(())
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::new("switch-to-buffer", switch_to_buffer),
        Command::new("kill-buffer", kill_buffer),
        Command::new("list-buffers", list_buffers),
    ]
}
