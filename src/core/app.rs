use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::widgets::ListState;
use std::process::Command;

use crate::utils::{Result, SshcError};
use crate::config::{parse_ssh_config, write_ssh_config, SshHost};
use crate::core::TerminalManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    ConfigManagement,
    EditingHost,
    ConfirmDelete,
    ConfirmDiscardEdit,
    ReviewChanges,
    ShowVersion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigAction {
    None,
    Add,
    Edit,
    Delete,
}

#[derive(Debug, Clone)]
pub struct EditingHostData {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub current_field: usize,
    pub original_name: String,
    pub original_hostname: String,
    pub original_user: String,
    pub original_port: String,
    pub original_identity_file: String,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Added(SshHost),
    Modified { old: SshHost, new: SshHost },
    Deleted(SshHost),
}

pub struct App {
    pub hosts: Vec<SshHost>,
    pub original_hosts: Vec<SshHost>,
    pub filtered_hosts: Vec<usize>,
    pub list_state: ListState,
    pub search_query: String,
    pub mode: AppMode,
    pub config_action: ConfigAction,
    pub editing_host: Option<EditingHostData>,
    pub editing_host_index: Option<usize>,
    pub pending_changes: Vec<ChangeType>,
    pub delete_target: Option<usize>,
    pub review_scroll: usize,
    pub current_edit_change_index: Option<usize>,
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
            original_hosts: hosts.clone(),
            hosts,
            filtered_hosts,
            list_state,
            search_query: String::new(),
            mode: AppMode::Normal,
            config_action: ConfigAction::None,
            editing_host: None,
            editing_host_index: None,
            pending_changes: Vec::new(),
            delete_target: None,
            review_scroll: 0,
            current_edit_change_index: None,
            should_quit: false,
        })
    }

    pub fn handle_event(&mut self, event: Event, terminal: &mut TerminalManager) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match self.mode {
                    AppMode::Search => self.handle_search_input(key.code, terminal)?,
                    AppMode::Normal => self.handle_normal_input(key.code, terminal)?,
                    AppMode::ConfigManagement => self.handle_config_input(key.code, terminal)?,
                    AppMode::EditingHost => self.handle_editing_input(key.code, terminal)?,
                    AppMode::ConfirmDelete => self.handle_delete_confirm_input(key.code)?,
                    AppMode::ConfirmDiscardEdit => self.handle_discard_edit_confirm_input(key.code)?,
                    AppMode::ReviewChanges => self.handle_review_input(key.code)?,
                    AppMode::ShowVersion => self.handle_version_input(key.code)?,
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
            KeyCode::Char('e') => self.mode = AppMode::ConfigManagement,
            KeyCode::Char('v') => self.mode = AppMode::ShowVersion,
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Enter => self.connect_to_selected(terminal)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_config_input(&mut self, key_code: KeyCode, _terminal: &mut TerminalManager) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                if !self.pending_changes.is_empty() {
                    self.mode = AppMode::ReviewChanges;
                } else {
                    self.mode = AppMode::Normal;
                    self.config_action = ConfigAction::None;
                }
            }
            KeyCode::Char('q') => {
                if !self.pending_changes.is_empty() {
                    self.mode = AppMode::ReviewChanges;
                } else {
                    self.mode = AppMode::Normal;
                }
            }
            KeyCode::Char('a') => {
                self.start_adding_host();
            }
            KeyCode::Char('e') => {
                self.start_editing_selected_host();
            }
            KeyCode::Char('d') => {
                self.start_deleting_selected_host();
            }
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
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
                        Ok(_) => {
                            // Force a complete redraw by clearing the terminal
                            terminal.terminal().clear().map_err(|e| SshcError::Terminal(e.to_string()))?;
                        },
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

    fn start_adding_host(&mut self) {
        let editing_data = EditingHostData {
            name: String::new(),
            hostname: String::new(),
            user: String::new(),
            port: String::new(),
            identity_file: String::new(),
            current_field: 0,
            original_name: String::new(),
            original_hostname: String::new(),
            original_user: String::new(),
            original_port: String::new(),
            original_identity_file: String::new(),
        };
        self.editing_host = Some(editing_data);
        self.editing_host_index = None;
        self.current_edit_change_index = None;
        self.mode = AppMode::EditingHost;
    }

    fn start_editing_selected_host(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(&host_idx) = self.filtered_hosts.get(selected) {
                if let Some(host) = self.hosts.get(host_idx) {
                    let name = host.name.clone();
                    let hostname = host.hostname.clone().unwrap_or_default();
                    let user = host.user.clone().unwrap_or_default();
                    let port = host.port.clone().unwrap_or_default();
                    let identity_file = host.identity_file.clone().unwrap_or_default();
                    
                    let editing_data = EditingHostData {
                        name: name.clone(),
                        hostname: hostname.clone(),
                        user: user.clone(),
                        port: port.clone(),
                        identity_file: identity_file.clone(),
                        current_field: 0,
                        original_name: name,
                        original_hostname: hostname,
                        original_user: user,
                        original_port: port,
                        original_identity_file: identity_file,
                    };
                    self.editing_host = Some(editing_data);
                    self.editing_host_index = Some(host_idx);
                    self.current_edit_change_index = None;
                    self.mode = AppMode::EditingHost;
                }
            }
        }
    }

    fn start_deleting_selected_host(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(&host_idx) = self.filtered_hosts.get(selected) {
                self.delete_target = Some(host_idx);
                self.mode = AppMode::ConfirmDelete;
            }
        }
    }

    fn handle_editing_input(&mut self, key_code: KeyCode, terminal: &mut TerminalManager) -> Result<()> {
        if let Some(ref mut editing_data) = self.editing_host {
            match key_code {
                KeyCode::Esc => {
                    if self.has_edit_changes() {
                        self.mode = AppMode::ConfirmDiscardEdit;
                    } else {
                        self.editing_host = None;
                        self.editing_host_index = None;
                        self.mode = AppMode::ConfigManagement;
                    }
                }
                KeyCode::Tab | KeyCode::Down => {
                    editing_data.current_field = (editing_data.current_field + 1) % 5;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    editing_data.current_field = if editing_data.current_field == 0 { 4 } else { editing_data.current_field - 1 };
                }
                KeyCode::Enter => {
                    self.save_edited_host();
                    terminal.terminal().clear().map_err(|e| SshcError::Terminal(e.to_string()))?;
                }
                KeyCode::Backspace => {
                    let field = match editing_data.current_field {
                        0 => &mut editing_data.name,
                        1 => &mut editing_data.hostname,
                        2 => &mut editing_data.user,
                        3 => &mut editing_data.port,
                        4 => &mut editing_data.identity_file,
                        _ => &mut editing_data.name,
                    };
                    field.pop();
                }
                KeyCode::Char(c) => {
                    let field = match editing_data.current_field {
                        0 => &mut editing_data.name,
                        1 => &mut editing_data.hostname,
                        2 => &mut editing_data.user,
                        3 => &mut editing_data.port,
                        4 => &mut editing_data.identity_file,
                        _ => &mut editing_data.name,
                    };
                    field.push(c);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_delete_confirm_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(host_idx) = self.delete_target {
                    if let Some(host) = self.hosts.get(host_idx).cloned() {
                        self.pending_changes.push(ChangeType::Deleted(host));
                        self.hosts.remove(host_idx);
                        self.filter_hosts();
                        
                        // Update selection
                        if let Some(selected) = self.list_state.selected() {
                            if self.filtered_hosts.is_empty() {
                                self.list_state.select(None);
                            } else if selected >= self.filtered_hosts.len() {
                                self.list_state.select(Some(self.filtered_hosts.len() - 1));
                            }
                        }
                    }
                }
                self.delete_target = None;
                self.mode = AppMode::ConfigManagement;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.delete_target = None;
                self.mode = AppMode::ConfigManagement;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_review_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.apply_changes()?;
                self.mode = AppMode::Normal;
                self.review_scroll = 0;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.discard_changes();
                self.mode = AppMode::Normal;
                self.review_scroll = 0;
            }
            KeyCode::Esc => {
                self.mode = AppMode::ConfigManagement;
                self.review_scroll = 0;
            }
            KeyCode::Up => {
                if self.review_scroll > 0 {
                    self.review_scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.review_scroll += 1;
            }
            KeyCode::PageUp => {
                self.review_scroll = self.review_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.review_scroll += 10;
            }
            _ => {}
        }
        Ok(())
    }

    fn save_edited_host(&mut self) {
        if let Some(editing_data) = &self.editing_host {
            if editing_data.name.trim().is_empty() {
                return;
            }

            let mut new_host = SshHost::new(editing_data.name.clone());
            if !editing_data.hostname.is_empty() {
                new_host.hostname = Some(editing_data.hostname.clone());
            }
            if !editing_data.user.is_empty() {
                new_host.user = Some(editing_data.user.clone());
            }
            if !editing_data.port.is_empty() {
                new_host.port = Some(editing_data.port.clone());
            }
            if !editing_data.identity_file.is_empty() {
                new_host.identity_file = Some(editing_data.identity_file.clone());
            }

            if let Some(host_idx) = self.editing_host_index {
                // Editing existing host
                if let Some(old_host) = self.hosts.get(host_idx).cloned() {
                    self.pending_changes.push(ChangeType::Modified { old: old_host, new: new_host.clone() });
                    self.current_edit_change_index = Some(self.pending_changes.len() - 1);
                    self.hosts[host_idx] = new_host;
                }
            } else {
                // Adding new host
                self.pending_changes.push(ChangeType::Added(new_host.clone()));
                self.current_edit_change_index = Some(self.pending_changes.len() - 1);
                self.hosts.push(new_host);
            }

            self.filter_hosts();
        }

        self.editing_host = None;
        self.editing_host_index = None;
        self.current_edit_change_index = None;
        self.mode = AppMode::ConfigManagement;
    }

    fn apply_changes(&mut self) -> Result<()> {
        write_ssh_config(&self.hosts).map_err(|e| SshcError::Config(e.to_string()))?;
        self.original_hosts = self.hosts.clone();
        self.pending_changes.clear();
        Ok(())
    }

    fn discard_changes(&mut self) {
        self.hosts = self.original_hosts.clone();
        self.pending_changes.clear();
        self.filter_hosts();
    }

    pub fn reload_config(&mut self) -> Result<()> {
        self.hosts = parse_ssh_config()?;
        self.original_hosts = self.hosts.clone();
        self.pending_changes.clear();
        self.filter_hosts();
        Ok(())
    }

    pub fn generate_diff_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        
        for change in &self.pending_changes {
            match change {
                ChangeType::Added(host) => {
                    lines.push(format!("+ Host {}", host.name));
                    if let Some(hostname) = &host.hostname {
                        lines.push(format!("+   HostName {}", hostname));
                    }
                    if let Some(user) = &host.user {
                        lines.push(format!("+   User {}", user));
                    }
                    if let Some(port) = &host.port {
                        lines.push(format!("+   Port {}", port));
                    }
                    if let Some(identity_file) = &host.identity_file {
                        lines.push(format!("+   IdentityFile {}", identity_file));
                    }
                    for (key, value) in &host.other_options {
                        lines.push(format!("+   {} {}", 
                            key.chars().next().unwrap().to_uppercase().chain(key.chars().skip(1)).collect::<String>(),
                            value));
                    }
                    lines.push(String::new());
                }
                ChangeType::Modified { old, new } => {
                    lines.push(format!("~ Host {}", old.name));
                    
                    // Compare each field
                    if old.hostname != new.hostname {
                        if let Some(old_hostname) = &old.hostname {
                            lines.push(format!("-   HostName {}", old_hostname));
                        }
                        if let Some(new_hostname) = &new.hostname {
                            lines.push(format!("+   HostName {}", new_hostname));
                        }
                    }
                    
                    if old.user != new.user {
                        if let Some(old_user) = &old.user {
                            lines.push(format!("-   User {}", old_user));
                        }
                        if let Some(new_user) = &new.user {
                            lines.push(format!("+   User {}", new_user));
                        }
                    }
                    
                    if old.port != new.port {
                        if let Some(old_port) = &old.port {
                            lines.push(format!("-   Port {}", old_port));
                        }
                        if let Some(new_port) = &new.port {
                            lines.push(format!("+   Port {}", new_port));
                        }
                    }
                    
                    if old.identity_file != new.identity_file {
                        if let Some(old_file) = &old.identity_file {
                            lines.push(format!("-   IdentityFile {}", old_file));
                        }
                        if let Some(new_file) = &new.identity_file {
                            lines.push(format!("+   IdentityFile {}", new_file));
                        }
                    }
                    
                    lines.push(String::new());
                }
                ChangeType::Deleted(host) => {
                    lines.push(format!("- Host {}", host.name));
                    if let Some(hostname) = &host.hostname {
                        lines.push(format!("-   HostName {}", hostname));
                    }
                    if let Some(user) = &host.user {
                        lines.push(format!("-   User {}", user));
                    }
                    if let Some(port) = &host.port {
                        lines.push(format!("-   Port {}", port));
                    }
                    if let Some(identity_file) = &host.identity_file {
                        lines.push(format!("-   IdentityFile {}", identity_file));
                    }
                    for (key, value) in &host.other_options {
                        lines.push(format!("-   {} {}", 
                            key.chars().next().unwrap().to_uppercase().chain(key.chars().skip(1)).collect::<String>(),
                            value));
                    }
                    lines.push(String::new());
                }
            }
        }
        
        lines
    }

    fn has_edit_changes(&self) -> bool {
        if let Some(editing_data) = &self.editing_host {
            editing_data.name != editing_data.original_name ||
            editing_data.hostname != editing_data.original_hostname ||
            editing_data.user != editing_data.original_user ||
            editing_data.port != editing_data.original_port ||
            editing_data.identity_file != editing_data.original_identity_file
        } else {
            false
        }
    }

    fn handle_discard_edit_confirm_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Discard changes and exit
                self.discard_current_edit();
                self.editing_host = None;
                self.editing_host_index = None;
                self.current_edit_change_index = None;
                self.mode = AppMode::ConfigManagement;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // Return to editing
                self.mode = AppMode::EditingHost;
            }
            _ => {}
        }
        Ok(())
    }

    fn discard_current_edit(&mut self) {
        // If there's a current edit change that was already saved, remove it and revert the hosts
        if let Some(change_index) = self.current_edit_change_index {
            if change_index < self.pending_changes.len() {
                match &self.pending_changes[change_index] {
                    ChangeType::Added(_) => {
                        // Remove the added host
                        if self.editing_host_index.is_some() {
                            // This is an edit, don't remove
                        } else {
                            // This was a new addition, remove the last host
                            self.hosts.pop();
                        }
                    }
                    ChangeType::Modified { old, .. } => {
                        // Revert to old host
                        if let Some(host_idx) = self.editing_host_index {
                            self.hosts[host_idx] = old.clone();
                        }
                    }
                    ChangeType::Deleted(_) => {
                        // This shouldn't happen in edit context
                    }
                }
                
                // Remove the change from pending_changes
                self.pending_changes.remove(change_index);
                self.filter_hosts();
            }
        }
        // Note: If current_edit_change_index is None, it means the user was editing
        // but never saved (never pressed Enter), so there's nothing to revert in
        // hosts or pending_changes - just clearing the editing state is sufficient
    }

    fn handle_version_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_version_info() -> VersionInfo {
        VersionInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            authors: env!("CARGO_PKG_AUTHORS").to_string(),
            license: env!("CARGO_PKG_LICENSE").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            repository: env!("CARGO_PKG_REPOSITORY").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub authors: String,
    pub license: String,
    pub description: String,
    pub repository: String,
}