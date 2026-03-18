use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::App;
use crate::search::fuzzy_search;

use super::status_bar;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let list_area = chunks[0];
    let status_area = chunks[1];

    let query = app.search_input.value().to_owned();
    let matcher = SkimMatcherV2::default();
    let matches = fuzzy_search(&matcher, &query, &app.items);

    let title = if query.is_empty() {
        format!(" GIFs ({}/{}) ", app.items.len(), app.items.len())
    } else {
        format!(" GIFs ({}/{}) ", matches.len(), app.items.len())
    };

    let items: Vec<ListItem> = matches
        .iter()
        .map(|m| {
            let item = &app.items[m.index];
            let spans = build_highlight_spans(&item.key, &m.matched_indices);
            ListItem::new(Line::from(spans))
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
    if !matches.is_empty() {
        let sel = app.search_selected.min(matches.len() - 1);
        state.select(Some(sel));
    }

    f.render_stateful_widget(list, list_area, &mut state);
    status_bar::draw_search(f, app, status_area);
}

fn build_highlight_spans<'a>(key: &'a str, matched_indices: &[usize]) -> Vec<Span<'a>> {
    let chars: Vec<char> = key.chars().collect();
    let mut spans = Vec::new();
    let mut current_normal = String::new();
    let mut current_highlight = String::new();
    let mut in_highlight = false;

    for (i, &ch) in chars.iter().enumerate() {
        let is_matched = matched_indices.contains(&i);
        match (in_highlight, is_matched) {
            (false, false) => current_normal.push(ch),
            (false, true) => {
                if !current_normal.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current_normal)));
                }
                current_highlight.push(ch);
                in_highlight = true;
            }
            (true, true) => current_highlight.push(ch),
            (true, false) => {
                if !current_highlight.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut current_highlight),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                current_normal.push(ch);
                in_highlight = false;
            }
        }
    }

    if !current_normal.is_empty() {
        spans.push(Span::raw(current_normal));
    }
    if !current_highlight.is_empty() {
        spans.push(Span::styled(
            current_highlight,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_all_normal() {
        let spans = build_highlight_spans("hello", &[]);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello");
    }

    #[test]
    fn highlight_all_matched() {
        let spans = build_highlight_spans("cat", &[0, 1, 2]);
        assert_eq!(spans.len(), 1);
        assert!(spans[0].style.fg == Some(Color::Yellow));
    }

    #[test]
    fn highlight_mixed() {
        // "cat" — match 'c' and 't' (indices 0 and 2)
        let spans = build_highlight_spans("cat", &[0, 2]);
        // c | a | t → highlighted | normal | highlighted → 3 spans
        assert!(spans.len() >= 2);
    }

    #[test]
    fn highlight_empty_string() {
        let spans = build_highlight_spans("", &[]);
        assert!(spans.is_empty());
    }
}
