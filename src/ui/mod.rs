use ratatui::{
    layout::{ Constraint, Direction, Layout, Margin },
    style::{ Color, Modifier, Style },
    text::{ Line, Span },
    widgets::{ Block, Borders, List, ListItem, Paragraph },
    Frame,
};

use crate::core::{ App, AppMode };

pub fn render(f: &mut Frame, app: &App) {
    match app.mode {
        AppMode::EditingHost => render_edit_form(f, app),
        AppMode::ConfirmDelete => render_delete_confirm(f, app),
        AppMode::ConfirmDiscardEdit => render_discard_edit_confirm(f, app),
        AppMode::ReviewChanges => render_changes_review(f, app),
        AppMode::ShowVersion => render_version_info(f, app),
        _ => render_main_view(f, app),
    }
}

fn render_main_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());

    render_search_box(f, app, chunks[0]);
    render_host_list(f, app, chunks[1]);
    render_help_text(f, app, chunks[1]);
}

fn render_search_box(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let search_text = match app.mode {
        AppMode::Search => format!("Search: {}|", app.search_query),
        AppMode::Normal => format!("Search: {} (Press / to search)", app.search_query),
        AppMode::ConfigManagement => {
            if !app.pending_changes.is_empty() {
                format!("Config Management Mode - {} pending changes", app.pending_changes.len())
            } else {
                "Config Management Mode".to_string()
            }
        }
        _ => "SSH Host Selector".to_string(),
    };

    let search_paragraph = Paragraph::new(search_text).block(
        Block::default().borders(Borders::ALL).title("SSH Host Selector")
    );

    f.render_widget(search_paragraph, area);
}

fn render_host_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.tree_items
        .iter()
        .map(|tree_item| {
            match tree_item {
                crate::core::TreeItem::Folder { name, expanded, .. } => {
                    let icon = if *expanded { "[-]" } else { "[+]" };
                    let folder_text = format!("{} {}", icon, name);
                    ListItem::new(Line::from(vec![
                        Span::styled(folder_text, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    ]))
                },
                crate::core::TreeItem::Host { host_index } => {
                    if let Some(host) = app.hosts.get(*host_index) {
                        let indent = if host.folder.is_some() { "  " } else { "" };
                        let display_text = format!("{}{}", indent, host.get_full_display_info());
                        ListItem::new(Line::from(vec![Span::raw(display_text)]))
                    } else {
                        ListItem::new(Line::from(vec![Span::raw("Invalid host")]))
                    }
                }
            }
        })
        .collect();

    let title = if app.search_query.is_empty() {
        "SSH Hosts (Enter/Space: Connect/Toggle folder, e: Edit)"
    } else {
        "Search Results"
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

fn render_help_text(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help_text = match app.mode {
        AppMode::Search => "ESC: Exit search | Enter/Space: Select and connect",
        AppMode::Normal => "↑↓: Select | Enter/Space: Connect/Toggle folder | /: Search | e: Edit config | v: Version | q: Quit",
        AppMode::ConfigManagement =>
            "a: Add host | e: Edit host | d: Delete host | q: Save & exit | ESC: Back",
        _ => "",
    };

    let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));

    let help_area = area.inner(
        &(Margin {
            vertical: 0,
            horizontal: 1,
        })
    );

    let help_y = help_area.bottom().saturating_sub(1);
    let help_rect = ratatui::layout::Rect {
        x: help_area.x,
        y: help_y,
        width: help_area.width,
        height: 1,
    };

    f.render_widget(help_paragraph, help_rect);
}

fn render_edit_form(f: &mut Frame, app: &App) {
    if let Some(editing_data) = &app.editing_host {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Name
                Constraint::Length(3), // Hostname
                Constraint::Length(3), // User
                Constraint::Length(3), // Port
                Constraint::Length(3), // Identity File
                Constraint::Length(3), // Folder
                Constraint::Length(3), // Display Name
                Constraint::Length(3), // Description
                Constraint::Length(3), // Visible
                Constraint::Min(1), // Help
            ])
            .split(f.size());

        let title = if app.editing_host_index.is_some() { "Edit Host" } else { "Add New Host" };
        let title_paragraph = Paragraph::new(title).block(Block::default().borders(Borders::ALL));
        f.render_widget(title_paragraph, chunks[0]);

        let fields = [
            ("Name", editing_data.name.as_str(), 0),
            ("Hostname", editing_data.hostname.as_str(), 1),
            ("User", editing_data.user.as_str(), 2),
            ("Port", editing_data.port.as_str(), 3),
            ("Identity File", editing_data.identity_file.as_str(), 4),
            ("Folder", editing_data.folder.as_str(), 5),
            ("Display Name *", editing_data.display_name.as_str(), 6),
            ("Description *", editing_data.description.as_str(), 7),
        ];

        for (i, (label, value, field_index)) in fields.iter().enumerate() {
            let style = if *field_index == editing_data.current_field {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default()
            };

            let paragraph = Paragraph::new(*value)
                .style(style)
                .block(Block::default().borders(Borders::ALL).title(*label));
            f.render_widget(paragraph, chunks[i + 1]);
        }

        // 可见性字段特殊处理
        let visible_style = if 8 == editing_data.current_field {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else {
            Style::default()
        };
        let visible_text = if editing_data.visible { "Yes" } else { "No" };
        let visible_paragraph = Paragraph::new(visible_text)
            .style(visible_style)
            .block(Block::default().borders(Borders::ALL).title("Visible on main page"));
        f.render_widget(visible_paragraph, chunks[8]);

        let help_text = "Tab/↑↓: Navigate | Enter: Save | ESC: Cancel | Space: Toggle visible | *=Optional";
        let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
        f.render_widget(help_paragraph, chunks[9]);
    }
}

fn render_delete_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 30, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    if let Some(host_idx) = app.delete_target {
        if let Some(host) = app.hosts.get(host_idx) {
            let text = format!("Delete host '{}'?\n\nThis action cannot be undone.", host.name);
            let paragraph = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Confirm Delete"))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(paragraph, area);

            let help_area = ratatui::layout::Rect {
                x: area.x + 1,
                y: area.bottom() - 2,
                width: area.width - 2,
                height: 1,
            };
            let help_text = "y: Yes, delete | n: No, cancel";
            let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
            f.render_widget(help_paragraph, help_area);
        }
    }
}

fn render_discard_edit_confirm(f: &mut Frame, _app: &App) {
    let area = centered_rect(50, 30, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let text = "You have unsaved changes.\n\nDiscard all changes and exit?";
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Discard Changes"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);

    let help_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.bottom() - 2,
        width: area.width - 2,
        height: 1,
    };
    let help_text = "y: Yes, discard changes | n: No, continue editing";
    let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
    f.render_widget(help_paragraph, help_area);
}

fn render_changes_review(f: &mut Frame, app: &App) {
    let area = centered_rect(90, 80, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let diff_lines = app.generate_diff_lines();

    // Calculate visible lines based on scroll position
    let content_height = (area.height as usize) - 4; // Account for borders and help text
    let start_line = app.review_scroll;
    let end_line = (start_line + content_height).min(diff_lines.len());

    let visible_lines: Vec<Line> = diff_lines[start_line..end_line]
        .iter()
        .map(|line| {
            if line.starts_with('+') {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else if line.starts_with('-') {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else if line.starts_with('~') {
                Line::from(Span::styled(line, Style::default().fg(Color::Yellow)))
            } else {
                Line::from(line.as_str())
            }
        })
        .collect();

    // Add header and footer information
    let mut all_lines = vec![
        Line::from(Span::styled("Pending Changes :", Style::default().fg(Color::Cyan))),
        Line::from("")
    ];

    all_lines.extend(visible_lines);

    // Add scrolling indicator
    if diff_lines.len() > content_height {
        let scroll_info = format!(
            "Lines {}-{} of {} (↑↓ to scroll, PgUp/PgDown for faster)",
            start_line + 1,
            end_line.min(diff_lines.len()),
            diff_lines.len()
        );
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(scroll_info, Style::default().fg(Color::Gray))));
    }

    all_lines.push(Line::from(""));
    all_lines.push(
        Line::from(Span::styled("Save these changes?", Style::default().fg(Color::White)))
    );

    let paragraph = Paragraph::new(all_lines)
        .block(Block::default().borders(Borders::ALL).title("Review Changes"))
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(paragraph, area);

    let help_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.bottom() - 2,
        width: area.width - 2,
        height: 1,
    };
    let help_text = "↑↓: Scroll | PgUp/PgDn: Fast scroll | y: Save | n: Discard | ESC: Back";
    let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
    f.render_widget(help_paragraph, help_area);
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_version_info(f: &mut Frame, _app: &App) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let version_info = App::get_version_info();
    
    let lines = vec![
        Line::from(Span::styled(
            format!("{}", version_info.name.to_uppercase()),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Version: {}", version_info.version),
            Style::default().fg(Color::Green)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Description: {}", version_info.description),
            Style::default().fg(Color::White)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Authors: {}", version_info.authors),
            Style::default().fg(Color::Yellow)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("License: {}", version_info.license),
            Style::default().fg(Color::Magenta)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Repository: {}", version_info.repository),
            Style::default().fg(Color::Blue)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "A Terminal User Interface for SSH connection management",
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("About"))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);

    let help_area = ratatui::layout::Rect {
        x: area.x + 1,
        y: area.bottom() - 2,
        width: area.width - 2,
        height: 1,
    };
    let help_text = "Press any key to continue";
    let help_paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(help_paragraph, help_area);
}
