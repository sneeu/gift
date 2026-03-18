use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::App;

use super::status_bar;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let status_area = chunks[1];

    // Title: "GIFs (N)" or "GIFs ⠋" while loading
    let title = if app.is_loading {
        format!(" GIFs {} ", app.spinner_char())
    } else {
        format!(" GIFs ({}) ", app.items.len())
    };

    let items: Vec<ListItem> = app
        .items
        .iter()
        .map(|item| {
            let size_kb = format!("{:.1} KB", item.size as f64 / 1024.0);
            // Truncate timestamp to date portion if it's long
            let ts = item.last_modified.get(..10).unwrap_or(&item.last_modified);
            let line = Line::from(vec![
                Span::raw(format!("{:<40}", truncate(&item.key, 40))),
                Span::styled(format!("{:>10}", size_kb), Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled(ts, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !app.items.is_empty() {
        state.select(Some(app.selected));
    }

    f.render_stateful_widget(list, list_area, &mut state);
    status_bar::draw_list(f, app, status_area);
}

fn truncate(s: &str, max_chars: usize) -> &str {
    let mut char_end = 0;
    for (i, (byte_idx, _)) in s.char_indices().enumerate() {
        if i >= max_chars {
            return &s[..char_end];
        }
        char_end = byte_idx;
    }
    // Also include the last character
    s
}
