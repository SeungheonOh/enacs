use std::env;
use std::path::PathBuf;
use std::time::Duration;

use enacs::frontend::{Frontend, TerminalFrontend};
use enacs::frontend::traits::FrontendEvent;
use enacs::state::EditorState;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut frontend = TerminalFrontend::new();
    frontend.init()?;

    let mut state = EditorState::new();

    let (width, height) = frontend.size();
    state.set_dimensions(width, height);

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let path = PathBuf::from(&args[1]);
        if let Err(e) = state.open_file(path) {
            state.message = Some(format!("Error opening file: {}", e));
        }
    }

    run_event_loop(&mut frontend, &mut state)?;

    frontend.shutdown()?;
    Ok(())
}

fn run_event_loop(
    frontend: &mut TerminalFrontend,
    state: &mut EditorState,
) -> anyhow::Result<()> {
    loop {
        frontend.render(state)?;

        if state.should_quit {
            break;
        }

        if let Some(event) = frontend.poll_event(Duration::from_millis(100)) {
            match event {
                FrontendEvent::Key(key) => {
                    state.handle_key(key);
                }
                FrontendEvent::Resize(width, height) => {
                    state.set_dimensions(width, height);
                }
                FrontendEvent::Mouse(_) => {
                }
                FrontendEvent::Focus(_) => {
                }
            }
        }
    }

    Ok(())
}
