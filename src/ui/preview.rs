use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;

use super::status_bar;

/// Split the screen into (left list, right preview pane, status bar).
pub(super) fn split(area: Rect) -> (Rect, Rect, Rect) {
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

    (panes[0], panes[1], status_area)
}

/// Draw the bordered preview pane shell; viuer fills the inner area post-draw.
pub(super) fn draw_pane(f: &mut Frame, app: &mut App, area: Rect, title: String, empty_text: &str) {
    // Store the rect for post-draw viuer rendering
    app.preview_pane_rect = Some(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(title);

    if app.preview_frames.is_empty() {
        f.render_widget(Paragraph::new(empty_text.to_owned()).block(block), area);
    } else {
        f.render_widget(block, area);
    }
}

/// Selectable list styled like the main list.
pub(super) fn list<'a>(items: Vec<ListItem<'a>>, title: String) -> List<'a> {
    List::new(items)
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
        .highlight_symbol("> ")
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let (left_area, right_area, status_area) = split(f.area());

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

    let list = list(list_items, title);

    let mut list_state = ListState::default();
    if !app.items.is_empty() {
        list_state.select(Some(selected_index));
    }
    f.render_stateful_widget(list, left_area, &mut list_state);

    let title = preview_title(app);
    draw_pane(f, app, right_area, title, "Loading…");

    status_bar::draw_preview(f, app, status_area);
}

fn preview_title(app: &App) -> String {
    app.active_item()
        .map(|i| format!(" {} ", i.key))
        .unwrap_or_else(|| " Preview ".to_owned())
}
