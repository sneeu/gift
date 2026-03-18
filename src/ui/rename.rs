use super::util::popup_area;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let area = popup_area(f.area(), 60, 7);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(" Rename GIF ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // From: label + value
            Constraint::Length(1), // gap
            Constraint::Length(1), // To: field
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // From (read-only)
    let from_line = Line::from(vec![
        Span::styled("From: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&app.rename_form.from, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(from_line), rows[1]);

    // To (editable)
    let value = app.rename_form.to.value();
    let cursor_ci = app.rename_form.to.cursor_char_index();
    let chars: Vec<char> = value.chars().collect();

    let before: String = chars[..cursor_ci.min(chars.len())].iter().collect();
    let cursor_char: String = chars
        .get(cursor_ci)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_owned());
    let after: String = if cursor_ci < chars.len() {
        chars[cursor_ci + 1..].iter().collect()
    } else {
        String::new()
    };

    let to_line = Line::from(vec![
        Span::styled("To:   ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::styled(before, Style::default().fg(Color::Yellow)),
        Span::styled(cursor_char, Style::default().bg(Color::Yellow).fg(Color::Black)),
        Span::styled(after, Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(to_line), rows[3]);

    let hint = Paragraph::new(Line::from(Span::styled(
        "Enter Confirm  Esc Cancel",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, rows[4]);
}

