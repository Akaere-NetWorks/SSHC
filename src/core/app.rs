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
    // 新增的元数据字段
    pub folder: String,
    pub display_name: String,
    pub description: String,
    pub visible: bool,
    pub current_field: usize,
    // 原始值用于比较变更
    pub original_name: String,
    pub original_hostname: String,
    pub original_user: String,
    pub original_port: String,
    pub original_identity_file: String,
    pub original_folder: String,
    pub original_display_name: String,
    pub original_description: String,
    pub original_visible: bool,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Added(SshHost),
    Modified { old: SshHost, new: SshHost },
    Deleted(SshHost),
}

#[derive(Debug, Clone)]
pub enum TreeItem {
    Folder { name: String, expanded: bool, children_indices: Vec<usize> },
    Host { host_index: usize },
}

pub struct App {
    pub hosts: Vec<SshHost>,
    pub original_hosts: Vec<SshHost>,
    pub filtered_hosts: Vec<usize>,
    pub tree_items: Vec<TreeItem>,  // 树形结构的展示项
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
        let list_state = ListState::default();
        
        let mut app = App {
            original_hosts: hosts.clone(),
            hosts,
            filtered_hosts,
            tree_items: Vec::new(),
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
        };
        
        app.rebuild_tree();
        
        if !app.tree_items.is_empty() {
            app.list_state.select(Some(0));
        }

        Ok(app)
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
                // 处理文件夹展开/收起或连接到主机
                if let Some(selected) = self.list_state.selected() {
                    if let Some(tree_item) = self.tree_items.get(selected) {
                        match tree_item {
                            TreeItem::Folder { .. } => {
                                self.toggle_folder_expanded(selected);
                            },
                            TreeItem::Host { .. } => {
                                self.connect_to_selected(terminal)?;
                            }
                        }
                    }
                }
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
            KeyCode::Enter | KeyCode::Char(' ') => {
                // 处理文件夹展开/收起或连接到主机
                if let Some(selected) = self.list_state.selected() {
                    if let Some(tree_item) = self.tree_items.get(selected) {
                        match tree_item {
                            TreeItem::Folder { .. } => {
                                self.toggle_folder_expanded(selected);
                            },
                            TreeItem::Host { .. } => {
                                self.connect_to_selected(terminal)?;
                            }
                        }
                    }
                }
            },
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
            self.rebuild_tree();
        } else {
            self.filtered_hosts = self
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, host)| host.matches_search(&self.search_query))
                .map(|(i, _)| i)
                .collect();
            
            // 在搜索模式下，显示简单列表而不是树形结构
            self.tree_items.clear();
            for &host_index in &self.filtered_hosts {
                self.tree_items.push(TreeItem::Host { host_index });
            }
        }
        
        if !self.tree_items.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn next(&mut self) {
        if self.tree_items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.tree_items.len() - 1 {
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
        if self.tree_items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tree_items.len() - 1
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
            if let Some(tree_item) = self.tree_items.get(selected) {
                match tree_item {
                    TreeItem::Host { host_index } => {
                        if let Some(host) = self.hosts.get(*host_index) {
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
                    },
                    TreeItem::Folder { .. } => {
                        // 文件夹项目不能连接，但可以展开/收起
                        // 这个逻辑会在按键处理中实现
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_selected_host(&self) -> Option<&SshHost> {
        self.list_state.selected()
            .and_then(|selected| self.tree_items.get(selected))
            .and_then(|tree_item| match tree_item {
                TreeItem::Host { host_index } => self.hosts.get(*host_index),
                TreeItem::Folder { .. } => None,
            })
    }

    fn start_adding_host(&mut self) {
        let editing_data = EditingHostData {
            name: String::new(),
            hostname: String::new(),
            user: String::new(),
            port: String::new(),
            identity_file: String::new(),
            folder: String::new(),
            display_name: String::new(),
            description: String::new(),
            visible: true,
            current_field: 0,
            original_name: String::new(),
            original_hostname: String::new(),
            original_user: String::new(),
            original_port: String::new(),
            original_identity_file: String::new(),
            original_folder: String::new(),
            original_display_name: String::new(),
            original_description: String::new(),
            original_visible: true,
        };
        self.editing_host = Some(editing_data);
        self.editing_host_index = None;
        self.current_edit_change_index = None;
        self.mode = AppMode::EditingHost;
    }

    fn start_editing_selected_host(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(tree_item) = self.tree_items.get(selected) {
                if let TreeItem::Host { host_index } = tree_item {
                    if let Some(host) = self.hosts.get(*host_index) {
                    let name = host.name.clone();
                    let hostname = host.hostname.clone().unwrap_or_default();
                    let user = host.user.clone().unwrap_or_default();
                    let port = host.port.clone().unwrap_or_default();
                    let identity_file = host.identity_file.clone().unwrap_or_default();
                    let folder = host.folder.clone().unwrap_or_default();
                    let display_name = host.display_name.clone().unwrap_or_default();
                    let description = host.description.clone().unwrap_or_default();
                    let visible = host.visible;
                    
                    let editing_data = EditingHostData {
                        name: name.clone(),
                        hostname: hostname.clone(),
                        user: user.clone(),
                        port: port.clone(),
                        identity_file: identity_file.clone(),
                        folder: folder.clone(),
                        display_name: display_name.clone(),
                        description: description.clone(),
                        visible,
                        current_field: 0,
                        original_name: name,
                        original_hostname: hostname,
                        original_user: user,
                        original_port: port,
                        original_identity_file: identity_file,
                        original_folder: folder,
                        original_display_name: display_name,
                        original_description: description,
                        original_visible: visible,
                    };
                    self.editing_host = Some(editing_data);
                    self.editing_host_index = Some(*host_index);
                    self.current_edit_change_index = None;
                    self.mode = AppMode::EditingHost;
                    }
                }
            }
        }
    }

    fn start_deleting_selected_host(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(tree_item) = self.tree_items.get(selected) {
                if let TreeItem::Host { host_index } = tree_item {
                    self.delete_target = Some(*host_index);
                    self.mode = AppMode::ConfirmDelete;
                }
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
                    editing_data.current_field = (editing_data.current_field + 1) % 9;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    editing_data.current_field = if editing_data.current_field == 0 { 8 } else { editing_data.current_field - 1 };
                }
                KeyCode::Enter => {
                    self.save_edited_host();
                    terminal.terminal().clear().map_err(|e| SshcError::Terminal(e.to_string()))?;
                }
                KeyCode::Backspace => {
                    match editing_data.current_field {
                        0 => { editing_data.name.pop(); },
                        1 => { editing_data.hostname.pop(); },
                        2 => { editing_data.user.pop(); },
                        3 => { editing_data.port.pop(); },
                        4 => { editing_data.identity_file.pop(); },
                        5 => { editing_data.folder.pop(); },
                        6 => { editing_data.display_name.pop(); },
                        7 => { editing_data.description.pop(); },
                        8 => { }, // 可见性字段不支持backspace
                        _ => {},
                    };
                }
                KeyCode::Char(' ') => {
                    if editing_data.current_field == 8 {
                        editing_data.visible = !editing_data.visible;
                    } else {
                        // 对其他字段添加空格
                        match editing_data.current_field {
                            0 => { editing_data.name.push(' '); },
                            1 => { editing_data.hostname.push(' '); },
                            2 => { editing_data.user.push(' '); },
                            3 => { editing_data.port.push(' '); },
                            4 => { editing_data.identity_file.push(' '); },
                            5 => { editing_data.folder.push(' '); },
                            6 => { editing_data.display_name.push(' '); },
                            7 => { editing_data.description.push(' '); },
                            _ => {},
                        };
                    }
                }
                KeyCode::Char(c) => {
                    match editing_data.current_field {
                        0 => { editing_data.name.push(c); },
                        1 => { editing_data.hostname.push(c); },
                        2 => { editing_data.user.push(c); },
                        3 => { editing_data.port.push(c); },
                        4 => { editing_data.identity_file.push(c); },
                        5 => { editing_data.folder.push(c); },
                        6 => { editing_data.display_name.push(c); },
                        7 => { editing_data.description.push(c); },
                        8 => { 
                            // 对于可见性字段，允许输入 t/f 或 y/n
                            match c.to_lowercase().next() {
                                Some('t') | Some('y') => editing_data.visible = true,
                                Some('f') | Some('n') => editing_data.visible = false,
                                _ => {},
                            }
                        },
                        _ => {},
                    };
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
                            if self.tree_items.is_empty() {
                                self.list_state.select(None);
                            } else if selected >= self.tree_items.len() {
                                self.list_state.select(Some(self.tree_items.len() - 1));
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
            
            // 设置元数据字段
            if !editing_data.folder.is_empty() {
                new_host.folder = Some(editing_data.folder.clone());
            }
            if !editing_data.display_name.is_empty() {
                new_host.display_name = Some(editing_data.display_name.clone());
            }
            if !editing_data.description.is_empty() {
                new_host.description = Some(editing_data.description.clone());
            }
            new_host.visible = editing_data.visible;

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
                    // 显示元数据注释
                    if let Some(folder) = &host.folder {
                        lines.push(format!("+ # @folder: {}", folder));
                    }
                    if let Some(display_name) = &host.display_name {
                        lines.push(format!("+ # @name: {}", display_name));
                    }
                    if let Some(description) = &host.description {
                        lines.push(format!("+ # @description: {}", description));
                    }
                    if !host.visible {
                        lines.push(format!("+ # @visible: false"));
                    }
                    
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
                    
                    // 比较元数据字段
                    if old.folder != new.folder {
                        if let Some(old_folder) = &old.folder {
                            lines.push(format!("- # @folder: {}", old_folder));
                        }
                        if let Some(new_folder) = &new.folder {
                            lines.push(format!("+ # @folder: {}", new_folder));
                        }
                    }
                    
                    if old.display_name != new.display_name {
                        if let Some(old_name) = &old.display_name {
                            lines.push(format!("- # @name: {}", old_name));
                        }
                        if let Some(new_name) = &new.display_name {
                            lines.push(format!("+ # @name: {}", new_name));
                        }
                    }
                    
                    if old.description != new.description {
                        if let Some(old_desc) = &old.description {
                            lines.push(format!("- # @description: {}", old_desc));
                        }
                        if let Some(new_desc) = &new.description {
                            lines.push(format!("+ # @description: {}", new_desc));
                        }
                    }
                    
                    if old.visible != new.visible {
                        lines.push(format!("- # @visible: {}", old.visible));
                        lines.push(format!("+ # @visible: {}", new.visible));
                    }
                    
                    // 比较基本SSH配置字段
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
                    // 显示被删除的元数据注释
                    if let Some(folder) = &host.folder {
                        lines.push(format!("- # @folder: {}", folder));
                    }
                    if let Some(display_name) = &host.display_name {
                        lines.push(format!("- # @name: {}", display_name));
                    }
                    if let Some(description) = &host.description {
                        lines.push(format!("- # @description: {}", description));
                    }
                    if !host.visible {
                        lines.push(format!("- # @visible: false"));
                    }
                    
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
            editing_data.identity_file != editing_data.original_identity_file ||
            editing_data.folder != editing_data.original_folder ||
            editing_data.display_name != editing_data.original_display_name ||
            editing_data.description != editing_data.original_description ||
            editing_data.visible != editing_data.original_visible
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

    pub fn get_available_folders(&self) -> Vec<String> {
        let mut folders: Vec<String> = self.hosts
            .iter()
            .filter_map(|host| host.folder.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        folders.sort();
        folders
    }

    pub fn rebuild_tree(&mut self) {
        self.tree_items.clear();
        
        // 按文件夹分组主机
        let mut folder_groups: std::collections::HashMap<Option<String>, Vec<usize>> = std::collections::HashMap::new();
        
        for (index, host) in self.hosts.iter().enumerate() {
            if !host.visible {
                continue; // 跳过不可见的主机
            }
            
            let folder_key = host.folder.clone();
            folder_groups.entry(folder_key).or_insert_with(Vec::new).push(index);
        }
        
        // 处理有文件夹的主机，按字母顺序排序
        let mut folder_names: Vec<String> = folder_groups.keys().filter_map(|k| k.clone()).collect();
        folder_names.sort();
        
        for folder_name in folder_names {
            if let Some(mut host_indices) = folder_groups.get(&Some(folder_name.clone())).cloned() {
                // 对文件夹内的主机也按名称排序
                host_indices.sort_by(|&a, &b| {
                    let name_a = self.hosts.get(a).map(|h| h.get_display_name()).unwrap_or_default();
                    let name_b = self.hosts.get(b).map(|h| h.get_display_name()).unwrap_or_default();
                    name_a.cmp(&name_b)
                });
                
                let folder_item = TreeItem::Folder {
                    name: folder_name,
                    expanded: true,  // 默认展开
                    children_indices: host_indices.clone(),
                };
                self.tree_items.push(folder_item);
                
                // 添加文件夹中的主机（只在展开状态下）
                for &host_index in &host_indices {
                    self.tree_items.push(TreeItem::Host { host_index });
                }
            }
        }
        
        // 处理根目录下的主机（没有文件夹的），按名称排序后添加
        if let Some(mut root_hosts) = folder_groups.remove(&None) {
            root_hosts.sort_by(|&a, &b| {
                let name_a = self.hosts.get(a).map(|h| h.get_display_name()).unwrap_or_default();
                let name_b = self.hosts.get(b).map(|h| h.get_display_name()).unwrap_or_default();
                name_a.cmp(&name_b)
            });
            
            for host_index in root_hosts {
                self.tree_items.push(TreeItem::Host { host_index });
            }
        }
    }

    pub fn toggle_folder_expanded(&mut self, folder_index: usize) {
        if let Some(&mut TreeItem::Folder { ref mut expanded, ref children_indices, .. }) = self.tree_items.get_mut(folder_index) {
            *expanded = !*expanded;
            
            if *expanded {
                // 展开：在文件夹后按排序顺序插入子项
                let mut children = children_indices.clone();
                children.sort_by(|&a, &b| {
                    let name_a = self.hosts.get(a).map(|h| h.get_display_name()).unwrap_or_default();
                    let name_b = self.hosts.get(b).map(|h| h.get_display_name()).unwrap_or_default();
                    name_a.cmp(&name_b)
                });
                
                for (i, &host_index) in children.iter().enumerate() {
                    self.tree_items.insert(folder_index + 1 + i, TreeItem::Host { host_index });
                }
            } else {
                // 收起：移除子项
                let children_count = children_indices.len();
                for _ in 0..children_count {
                    if folder_index + 1 < self.tree_items.len() {
                        self.tree_items.remove(folder_index + 1);
                    }
                }
            }
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