use std::fmt;

#[derive(Debug)]
pub enum SshcError {
    Io(std::io::Error),
    Config(String),
    Terminal(String),
    Ssh(String),
}

impl fmt::Display for SshcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshcError::Io(err) => write!(f, "IO error: {}", err),
            SshcError::Config(msg) => write!(f, "Config error: {}", msg),
            SshcError::Terminal(msg) => write!(f, "Terminal error: {}", msg),
            SshcError::Ssh(msg) => write!(f, "SSH error: {}", msg),
        }
    }
}

impl std::error::Error for SshcError {}

impl From<std::io::Error> for SshcError {
    fn from(err: std::io::Error) -> Self {
        SshcError::Io(err)
    }
}

impl From<anyhow::Error> for SshcError {
    fn from(err: anyhow::Error) -> Self {
        SshcError::Config(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SshcError>;