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
}

impl SshHost {
    pub fn display_name(&self) -> String {
        match (&self.hostname, &self.user) {
            (Some(host), Some(user)) => format!("{} ({}@{})", self.name, user, host),
            (Some(host), None) => format!("{} ({})", self.name, host),
            _ => self.name.clone(),
        }
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query) ||
            self.hostname.as_ref().map_or(false, |h| h.to_lowercase().contains(&query)) ||
            self.user.as_ref().map_or(false, |u| u.to_lowercase().contains(&query))
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

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }

        let key = parts[0].to_lowercase();
        let value = parts[1].trim();

        match key.as_str() {
            "host" => {
                if let Some(host) = current_host.take() {
                    hosts.push(host);
                }
                current_host = Some(SshHost {
                    name: value.to_string(),
                    hostname: None,
                    user: None,
                    port: None,
                    identity_file: None,
                    other_options: HashMap::new(),
                });
            }
            "hostname" => {
                if let Some(ref mut host) = current_host {
                    host.hostname = Some(value.to_string());
                }
            }
            "user" => {
                if let Some(ref mut host) = current_host {
                    host.user = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(ref mut host) = current_host {
                    host.port = Some(value.to_string());
                }
            }
            "identityfile" => {
                if let Some(ref mut host) = current_host {
                    host.identity_file = Some(value.to_string());
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
