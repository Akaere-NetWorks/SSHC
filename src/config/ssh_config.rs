use anyhow::{ Context, Result };
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct SshHost {
    pub name: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<String>,
    pub identity_file: Option<String>,
    pub other_options: HashMap<String, String>,
    // 元数据字段
    pub folder: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub visible: bool,
}

impl SshHost {
    pub fn new(name: String) -> Self {
        Self {
            name,
            hostname: None,
            user: None,
            port: None,
            identity_file: None,
            other_options: HashMap::new(),
            folder: None,
            display_name: None,
            description: None,
            visible: true,
        }
    }

    pub fn get_display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn get_full_display_info(&self) -> String {
        let display_name = self.get_display_name();
        let mut info_parts = Vec::new();

        if let Some(user) = &self.user {
            if let Some(hostname) = &self.hostname {
                info_parts.push(format!("{}@{}", user, hostname));
            } else {
                info_parts.push(format!("user:{}", user));
            }
        } else if let Some(hostname) = &self.hostname {
            info_parts.push(hostname.clone());
        }

        if let Some(port) = &self.port {
            info_parts.push(format!("port:{}", port));
        }

        let base_info = if !info_parts.is_empty() {
            format!("{} ({})", display_name, info_parts.join(" "))
        } else {
            display_name
        };

        if let Some(description) = &self.description {
            format!("{} # {}", base_info, description)
        } else {
            base_info
        }
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query) ||
            self.hostname.as_ref().map_or(false, |h| h.to_lowercase().contains(&query)) ||
            self.user.as_ref().map_or(false, |u| u.to_lowercase().contains(&query)) ||
            self.display_name.as_ref().map_or(false, |d| d.to_lowercase().contains(&query)) ||
            self.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query)) ||
            self.folder.as_ref().map_or(false, |f| f.to_lowercase().contains(&query))
    }
}

pub fn parse_ssh_config() -> Result<Vec<SshHost>> {
    let home_dir = home::home_dir().context("Unable to get user home directory")?;
    let config_path = home_dir.join(".ssh").join("config");

    if !config_path.exists() {
        return Ok(vec![]);
    }

    let content = fs
        ::read_to_string(&config_path)
        .with_context(|| format!("Unable to read SSH config file: {:?}", config_path))?;

    let mut hosts = Vec::new();
    let mut current_host: Option<SshHost> = None;
    let mut pending_metadata: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // 处理元数据注释
        if line.starts_with('#') {
            if line.starts_with("# @") {
                let meta_line = &line[3..].trim();
                if let Some(colon_pos) = meta_line.find(':') {
                    let key = meta_line[..colon_pos].trim().to_string();
                    let value = meta_line[colon_pos + 1..].trim().to_string();
                    pending_metadata.insert(key, value);
                }
            }
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            continue;
        }

        let key = parts[0].to_lowercase();
        let value = if parts.len() > 1 { parts[1].trim() } else { "" };

        match key.as_str() {
            "host" => {
                if let Some(host) = current_host.take() {
                    hosts.push(host);
                }
                let mut new_host = SshHost::new(value.to_string());

                // 应用待处理的元数据
                if let Some(folder) = pending_metadata.remove("folder") {
                    new_host.folder = Some(folder);
                }
                if let Some(display_name) = pending_metadata.remove("name") {
                    new_host.display_name = Some(display_name);
                }
                if let Some(description) = pending_metadata.remove("description") {
                    new_host.description = Some(description);
                }
                if let Some(visible) = pending_metadata.remove("visible") {
                    new_host.visible = visible.to_lowercase() != "false";
                }

                pending_metadata.clear();
                current_host = Some(new_host);
            }
            "hostname" => {
                if let Some(ref mut host) = current_host {
                    if !value.is_empty() {
                        host.hostname = Some(value.to_string());
                    }
                }
            }
            "user" => {
                if let Some(ref mut host) = current_host {
                    if !value.is_empty() {
                        host.user = Some(value.to_string());
                    }
                }
            }
            "port" => {
                if let Some(ref mut host) = current_host {
                    if !value.is_empty() {
                        host.port = Some(value.to_string());
                    }
                }
            }
            "identityfile" => {
                if let Some(ref mut host) = current_host {
                    if !value.is_empty() {
                        host.identity_file = Some(value.to_string());
                    }
                }
            }
            _ => {
                if let Some(ref mut host) = current_host {
                    host.other_options.insert(key, value.to_string());
                }
            }
        }
    }

    if let Some(host) = current_host {
        hosts.push(host);
    }

    Ok(hosts)
}

pub fn write_ssh_config(hosts: &[SshHost]) -> Result<()> {
    let home_dir = home::home_dir().context("Unable to get user home directory")?;
    let config_path = home_dir.join(".ssh").join("config");

    // Create .ssh directory if it doesn't exist
    let ssh_dir = home_dir.join(".ssh");
    if !ssh_dir.exists() {
        std::fs
            ::create_dir_all(&ssh_dir)
            .with_context(|| format!("Unable to create .ssh directory: {:?}", ssh_dir))?;
    }

    let mut content = String::new();

    for host in hosts {
        // 写入元数据注释
        if let Some(folder) = &host.folder {
            content.push_str(&format!("# @folder: {}\n", folder));
        }
        if let Some(display_name) = &host.display_name {
            content.push_str(&format!("# @name: {}\n", display_name));
        }
        if let Some(description) = &host.description {
            content.push_str(&format!("# @description: {}\n", description));
        }
        if !host.visible {
            content.push_str("# @visible: false\n");
        }

        content.push_str(&format!("Host {}\n", host.name));

        if let Some(hostname) = &host.hostname {
            content.push_str(&format!("    HostName {}\n", hostname));
        }
        if let Some(user) = &host.user {
            content.push_str(&format!("    User {}\n", user));
        }
        if let Some(port) = &host.port {
            content.push_str(&format!("    Port {}\n", port));
        }
        if let Some(identity_file) = &host.identity_file {
            content.push_str(&format!("    IdentityFile {}\n", identity_file));
        }

        for (key, value) in &host.other_options {
            content.push_str(
                &format!(
                    "    {} {}\n",
                    key
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .chain(key.chars().skip(1))
                        .collect::<String>(),
                    value
                )
            );
        }

        content.push('\n');
    }

    std::fs
        ::write(&config_path, content)
        .with_context(|| format!("Unable to write SSH config file: {:?}", config_path))?;

    Ok(())
}
