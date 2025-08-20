use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::widgets::ListState;
use std::process::Command;

use crate::error::{Result, SshcError};
use crate::ssh_config::{parse_ssh_config, SshHost};
use crate::terminal::TerminalManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
}

pub struct App {
    pub hosts: Vec<SshHost>,
    pub filtered_hosts: Vec<usize>,
    pub list_state: ListState,
    pub search_query: String,
    pub mode: AppMode,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let hosts = parse_ssh_config()?;
        let filtered_hosts: Vec<usize> = (0..hosts.len()).collect();
        let mut list_state = ListState::default();
        
        if !hosts.is_empty() {
            list_state.select(Some(0));
        }

        Ok(App {
            hosts,
            filtered_hosts,
            list_state,
            search_query: String::new(),
            mode: AppMode::Normal,
            should_quit: false,
        })
    }

    pub fn handle_event(&mut self, event: Event, terminal: &mut TerminalManager) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match self.mode {
                    AppMode::Search => self.handle_search_input(key.code, terminal)?,
                    AppMode::Normal => self.handle_normal_input(key.code, terminal)?,
                }
            }
        }
        Ok(())
    }

    fn handle_search_input(&mut self, key_code: KeyCode, terminal: &mut TerminalManager) -> Result<()> {
        match key_code {
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.filter_hosts();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.filter_hosts();
            }
            KeyCode::Enter => {
                self.mode = AppMode::Normal;
                self.connect_to_selected(terminal)?;
            }
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key_code: KeyCode, terminal: &mut TerminalManager) -> Result<()> {
        match key_code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('/') => self.mode = AppMode::Search,
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Enter => self.connect_to_selected(terminal)?,
            _ => {}
        }
        Ok(())
    }

    pub fn filter_hosts(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_hosts = (0..self.hosts.len()).collect();
        } else {
            self.filtered_hosts = self
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, host)| host.matches_search(&self.search_query))
                .map(|(i, _)| i)
                .collect();
        }
        
        if !self.filtered_hosts.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn next(&mut self) {
        if self.filtered_hosts.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_hosts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.filtered_hosts.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_hosts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn connect_to_selected(&self, terminal: &mut TerminalManager) -> Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(&host_idx) = self.filtered_hosts.get(selected) {
                if let Some(host) = self.hosts.get(host_idx) {
                    terminal.suspend()?;
                    
                    let status = Command::new("ssh")
                        .arg(&host.name)
                        .status();
                    
                    terminal.resume()?;
                    
                    match status {
                        Ok(_) => (),
                        Err(e) => return Err(SshcError::Ssh(format!("SSH connection error: {}", e))),
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_host(&self) -> Option<&SshHost> {
        self.list_state.selected()
            .and_then(|selected| self.filtered_hosts.get(selected))
            .and_then(|&host_idx| self.hosts.get(host_idx))
    }
}