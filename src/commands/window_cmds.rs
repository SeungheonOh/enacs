use super::registry::{Command, CommandContext, CommandResult};
use crate::state::EditorState;

pub fn split_window_below(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.windows.split_vertical();
    state.message = Some("Window split vertically".to_string());
    Ok(())
}

pub fn split_window_right(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.windows.split_horizontal();
    state.message = Some("Window split horizontally".to_string());
    Ok(())
}

pub fn delete_window(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    if state.windows.count() > 1 {
        state.windows.delete_current();
        state.message = Some("Window deleted".to_string());
    } else {
        state.message = Some("Cannot delete the only window".to_string());
    }
    Ok(())
}

pub fn delete_other_windows(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.windows.delete_others();
    state.message = Some("Deleted other windows".to_string());
    Ok(())
}

pub fn other_window(state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
    state.windows.cycle_next();
    Ok(())
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::new("split-window-below", split_window_below),
        Command::new("split-window-right", split_window_right),
        Command::new("delete-window", delete_window),
        Command::new("delete-other-windows", delete_other_windows),
        Command::new("other-window", other_window),
    ]
}
