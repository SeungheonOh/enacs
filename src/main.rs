use std::env;
use std::path::PathBuf;

use enacs::frontend::{Frontend, TerminalFrontend};
use enacs::state::EditorState;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut state = EditorState::new();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] != "--gui" {
        let path = PathBuf::from(&args[1]);
        if let Err(e) = state.open_file(path) {
            state.message = Some(format!("Error opening file: {}", e));
        }
    }

    if args.iter().any(|a| a == "--gui") {
        let mut frontend = enacs::frontend::GuiFrontend::new();
        frontend.init()?;
        frontend.run(state)?;
    } else {
        let mut frontend = TerminalFrontend::new();
        frontend.init()?;
        let (width, height) = frontend.size();
        state.set_dimensions(width, height);
        frontend.run(state)?;
    }

    Ok(())
}
