pub mod app;
pub mod error;
pub mod ssh_config;
pub mod terminal;
pub mod ui;

use crossterm::event;

use crate::app::App;
use crate::error::Result;
use crate::terminal::TerminalManager;
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