use std::collections::HashMap;
use thiserror::Error;

use crate::state::EditorState;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),

    #[error("Buffer is read-only")]
    ReadOnly,

    #[error("No mark set")]
    NoMark,

    #[error("No match")]
    NoMatch,

    #[error("Cancelled")]
    Cancelled,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type CommandResult = Result<(), CommandError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixArg {
    None,
    Universal(i32),
    Negative,
    Raw(i32),
}

impl Default for PrefixArg {
    fn default() -> Self {
        PrefixArg::None
    }
}

impl PrefixArg {
    pub fn count(&self) -> i32 {
        match self {
            PrefixArg::None => 1,
            PrefixArg::Universal(n) => *n,
            PrefixArg::Negative => -1,
            PrefixArg::Raw(n) => *n,
        }
    }

    pub fn is_set(&self) -> bool {
        !matches!(self, PrefixArg::None)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandContext {
    pub prefix_arg: PrefixArg,
    pub last_command: Option<&'static str>,
}

impl CommandContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_prefix(prefix: PrefixArg) -> Self {
        Self {
            prefix_arg: prefix,
            last_command: None,
        }
    }

    pub fn count(&self) -> i32 {
        self.prefix_arg.count()
    }

    pub fn repeat_count(&self) -> usize {
        self.prefix_arg.count().unsigned_abs() as usize
    }
}

pub type CommandFn = fn(&mut EditorState, &CommandContext) -> CommandResult;

#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub execute: CommandFn,
    pub is_kill: bool,
    pub preserves_mark: bool,
    pub breaks_undo_coalesce: bool,
}

impl Command {
    pub const fn new(name: &'static str, execute: CommandFn) -> Self {
        Self {
            name,
            execute,
            is_kill: false,
            preserves_mark: false,
            breaks_undo_coalesce: true,
        }
    }

    pub const fn kill(name: &'static str, execute: CommandFn) -> Self {
        Self {
            name,
            execute,
            is_kill: true,
            preserves_mark: false,
            breaks_undo_coalesce: true,
        }
    }

    pub const fn motion(name: &'static str, execute: CommandFn) -> Self {
        Self {
            name,
            execute,
            is_kill: false,
            preserves_mark: true,
            breaks_undo_coalesce: true,
        }
    }

    pub const fn mark(name: &'static str, execute: CommandFn) -> Self {
        Self {
            name,
            execute,
            is_kill: false,
            preserves_mark: true,
            breaks_undo_coalesce: false,
        }
    }

    pub const fn editing(name: &'static str, execute: CommandFn) -> Self {
        Self {
            name,
            execute,
            is_kill: false,
            preserves_mark: false,
            breaks_undo_coalesce: false,
        }
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("is_kill", &self.is_kill)
            .field("preserves_mark", &self.preserves_mark)
            .finish()
    }
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<&'static str, Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name, command);
    }

    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn execute(
        &self,
        name: &str,
        state: &mut EditorState,
        ctx: &CommandContext,
    ) -> CommandResult {
        let command = self
            .commands
            .get(name)
            .ok_or_else(|| CommandError::NotFound(name.to_string()))?;

        (command.execute)(state, ctx)
    }

    pub fn names(&self) -> impl Iterator<Item = &&'static str> {
        self.commands.keys()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

pub fn build_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    for cmd in super::motion::all_commands() {
        registry.register(cmd);
    }

    for cmd in super::editing::all_commands() {
        registry.register(cmd);
    }

    for cmd in super::kill_yank::all_commands() {
        registry.register(cmd);
    }

    for cmd in super::buffer_cmds::all_commands() {
        registry.register(cmd);
    }

    for cmd in super::file_cmds::all_commands() {
        registry.register(cmd);
    }

    for cmd in super::window_cmds::all_commands() {
        registry.register(cmd);
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_command(_state: &mut EditorState, _ctx: &CommandContext) -> CommandResult {
        Ok(())
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = CommandRegistry::new();
        registry.register(Command::new("test-command", dummy_command));

        assert!(registry.get("test-command").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_prefix_arg_count() {
        assert_eq!(PrefixArg::None.count(), 1);
        assert_eq!(PrefixArg::Universal(4).count(), 4);
        assert_eq!(PrefixArg::Negative.count(), -1);
        assert_eq!(PrefixArg::Raw(10).count(), 10);
    }
}
