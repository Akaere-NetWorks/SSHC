# SSH-TUI

A Terminal User Interface for SSH connection management written in Rust.

## Features

- **Browse SSH Hosts**: View all configured SSH hosts from your `~/.ssh/config` file
- **Quick Search**: Search hosts by name, hostname, or username with `/` key
- **One-Click Connect**: Connect to hosts with Enter key using system SSH
- **Config Management**: Add, edit, and delete SSH host configurations
- **Git-style Diff**: Review configuration changes before saving
- **Port Display**: Shows all SSH parameters including port numbers
- **Version Info**: Display project information with `v` key

## Installation

### From Source

```bash
git clone https://github.com/Akaere-NetWorks/SSHC.git
cd SSHC
cargo build --release
```

The binary will be available at `target/release/ssh-tui`.

## Usage

### Basic Navigation

```bash
ssh-tui
```

**Normal Mode:**
- `↑↓`: Navigate host list
- `Enter`: Connect to selected host
- `/`: Search hosts
- `e`: Enter config management mode
- `v`: Show version information
- `q`: Quit

**Search Mode:**
- Type to search hosts
- `ESC`: Exit search
- `Enter`: Connect to selected host

### Configuration Management

Press `e` to enter configuration management mode:

- `a`: Add new host
- `e`: Edit selected host
- `d`: Delete selected host
- `q`: Save changes and exit
- `ESC`: Back to normal mode (with discard confirmation if changes exist)

### Host Configuration

When adding or editing hosts, configure:
- **Name**: Host alias
- **Hostname**: Server address
- **User**: SSH username
- **Port**: SSH port (default: 22)
- **Identity File**: Path to SSH key

Navigate fields with `Tab` or `↑↓`, save with `Enter`, cancel with `ESC`.

### Review Changes

Before saving, review all changes in a git-style diff view:
- Green lines: Added configurations
- Red lines: Removed configurations
- Yellow lines: Modified configurations
- `↑↓`: Scroll through changes
- `PgUp/PgDn`: Fast scroll
- `y`: Save changes
- `n`: Discard changes
- `ESC`: Back to editing

## Configuration File

SSH-TUI reads and writes to the standard SSH configuration file at `~/.ssh/config`. The format follows OpenSSH standards:

```
Host myserver
    HostName example.com
    User ubuntu
    Port 2222
    IdentityFile ~/.ssh/my_key
```

## Requirements

- Rust 1.70+ (for building from source)
- SSH client installed on system
- `~/.ssh/config` file (will be created if doesn't exist)

## License

This project is licensed under the AGPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## Author

**Akaere Networks**

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Troubleshooting

### Terminal Rendering Issues

If the interface doesn't render correctly after SSH connections:
- The application includes proper terminal state management
- Terminal is automatically restored after SSH session ends

### Permission Issues

Ensure you have read/write permissions for:
- `~/.ssh/config` file
- `~/.ssh/` directory

### SSH Connection Problems

- Verify SSH client is installed and accessible
- Check SSH host configurations are valid
- Ensure SSH keys have proper permissions (600)

## Keyboard Shortcuts Reference

| Key | Mode | Action |
|-----|------|--------|
| `↑↓` | Normal/Config | Navigate list |
| `Enter` | Normal | Connect to host |
| `Enter` | Edit | Save changes |
| `/` | Normal | Start search |
| `e` | Normal | Config management |
| `a` | Config | Add host |
| `e` | Config | Edit host |
| `d` | Config | Delete host |
| `v` | Normal | Version info |
| `q` | Normal/Config | Quit/Save & exit |
| `ESC` | Any | Cancel/Back |
| `Tab` | Edit | Next field |
| `PgUp/PgDn` | Review | Fast scroll |