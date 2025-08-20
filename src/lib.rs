pub mod core;
pub mod config;
pub mod ui;
pub mod utils;

use crossterm::event;

use crate::core::{ App, TerminalManager };
use crate::utils::Result;
use crate::ui::render;

pub fn run() -> Result<()> {
    let mut terminal = TerminalManager::new()?;
    let mut app = App::new()?;

    let result = run_app(&mut terminal, &mut app);
    terminal.restore()?;

    result
}

fn run_app(terminal: &mut TerminalManager, app: &mut App) -> Result<()> {
    loop {
        terminal.terminal().draw(|f| render(f, app))?;

        if app.should_quit {
            break;
        }

        let event = event::read()?;
        app.handle_event(event, terminal)?;
    }

    Ok(())
}
