use ratatui::{
    layout::{ Constraint, Direction, Layout, Margin },
    style::{ Color, Modifier, Style },
    text::{ Line, Span },
    widgets::{ Block, Borders, List, ListItem, Paragraph },
    Frame,
};

use crate::app::{ App, AppMode };

pub fn render(f: &mut Frame, app: &App) {
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
    };

    let search_paragraph = Paragraph::new(search_text).block(
        Block::default().borders(Borders::ALL).title("SSH Host Selector")
    );

    f.render_widget(search_paragraph, area);
}

fn render_host_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.filtered_hosts
        .iter()
        .map(|&i| {
            let host = &app.hosts[i];
            ListItem::new(Line::from(vec![Span::raw(host.display_name())]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Host List"))
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

fn render_help_text(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help_text = match app.mode {
        AppMode::Search => "ESC: Exit search | Enter: Select and connect",
        AppMode::Normal => "↑↓: Select | Enter: Connect | /: Search | q: Quit",
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
