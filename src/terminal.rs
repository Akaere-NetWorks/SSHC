use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::error::{Result, SshcError};

pub struct TerminalManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalManager {
    pub fn new() -> Result<Self> {
        enable_raw_mode().map_err(|e| SshcError::Terminal(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| SshcError::Terminal(e.to_string()))?;
        
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| SshcError::Terminal(e.to_string()))?;

        Ok(TerminalManager { terminal })
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    pub fn suspend(&mut self) -> Result<()> {
        disable_raw_mode().map_err(|e| SshcError::Terminal(e.to_string()))?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(|e| SshcError::Terminal(e.to_string()))?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        enable_raw_mode().map_err(|e| SshcError::Terminal(e.to_string()))?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        ).map_err(|e| SshcError::Terminal(e.to_string()))?;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode().map_err(|e| SshcError::Terminal(e.to_string()))?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(|e| SshcError::Terminal(e.to_string()))?;
        self.terminal.show_cursor()
            .map_err(|e| SshcError::Terminal(e.to_string()))?;
        Ok(())
    }
}