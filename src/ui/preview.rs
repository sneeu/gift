use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;

use super::status_bar;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let width = area.width;

    // Responsive split: 28% at ≥160, 33% at ≥120, 40% otherwise
    let left_pct = if width >= 160 { 28 } else if width >= 120 { 33 } else { 40 };
    let right_pct = 100 - left_pct;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let content_area = chunks[0];
    let status_area = chunks[1];

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(right_pct),
        ])
        .split(content_area);

    let left_area = panes[0];
    let right_area = panes[1];

    // Store the right pane rect for post-draw viuer rendering
    app.preview_pane_rect = Some(right_area);

    // Left pane: show search results when coming from search, full list otherwise.
    let from_search = app.prev_mode == crate::app::AppMode::Search;

    let (list_items, selected_index, title) = if from_search {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .map(|&i| ListItem::new(Line::from(Span::raw(&app.items[i].key))))
            .collect();
        let title = format!(" GIFs ({}) ", app.search_results.len());
        (items, app.search_selected, title)
    } else {
        let items: Vec<ListItem> = app
            .items
            .iter()
            .map(|item| ListItem::new(Line::from(Span::raw(&item.key))))
            .collect();
        let title = format!(" GIFs ({}) ", app.items.len());
        (items, app.selected, title)
    };

    let list = List::new(list_items)
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

    let mut list_state = ListState::default();
    if !app.items.is_empty() {
        list_state.select(Some(selected_index));
    }
    f.render_stateful_widget(list, left_area, &mut list_state);

    // Right pane: preview area (viuer renders here post-draw)
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(preview_title(app));

    if app.preview_frames.is_empty() {
        let loading = Paragraph::new("Loading…").block(preview_block);
        f.render_widget(loading, right_area);
    } else {
        // Render the block shell — viuer will fill the inner area post-draw
        f.render_widget(preview_block, right_area);
    }

    status_bar::draw_preview(f, app, status_area);
}

fn preview_title(app: &App) -> String {
    app.active_item()
        .map(|i| format!(" {} ", i.key))
        .unwrap_or_else(|| " Preview ".to_owned())
}
