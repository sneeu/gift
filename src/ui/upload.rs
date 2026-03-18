use super::util::popup_area;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, UploadField};

pub fn draw(f: &mut Frame, app: &App) {
    let area = popup_area(f.area(), 60, 12);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Upload GIF ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(2), // source label + field
            Constraint::Length(1), // gap
            Constraint::Length(2), // name label + field
            Constraint::Length(1), // padding
            Constraint::Length(1), // submit hint
        ])
        .split(inner);

    let source_label = if app.upload_form.source.value().starts_with("http") {
        "URL:"
    } else {
        "File:"
    };

    draw_field(
        f,
        rows[1],
        source_label,
        app.upload_form.source.value(),
        app.upload_form.source.cursor_char_index(),
        app.upload_form.focused == UploadField::Source,
    );

    draw_field(
        f,
        rows[3],
        "Name:",
        app.upload_form.name.value(),
        app.upload_form.name.cursor_char_index(),
        app.upload_form.focused == UploadField::Name,
    );

    let hint = Paragraph::new(Line::from(Span::styled(
        "Tab Switch field  Enter Submit  Esc Cancel",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(hint, rows[5]);
}

fn draw_field(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    cursor_ci: usize,
    focused: bool,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let label_style = if focused {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(label, label_style))),
        rows[0],
    );

    let field_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let chars: Vec<char> = value.chars().collect();
    let mut spans: Vec<Span> = Vec::new();

    if focused {
        // Render text with a block cursor at the cursor position
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
        spans.push(Span::styled(before, field_style));
        spans.push(Span::styled(
            cursor_char,
            Style::default().bg(Color::Yellow).fg(Color::Black),
        ));
        spans.push(Span::styled(after, field_style));
    } else {
        spans.push(Span::styled(value.to_owned(), field_style));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}

