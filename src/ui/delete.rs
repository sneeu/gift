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
    let area = popup_area(f.area(), 50, 7);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .title(" Delete ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let name = app
        .active_item()
        .map(|i| i.key.as_str())
        .unwrap_or("(unknown)");

    let question = Paragraph::new(Line::from(vec![
        Span::raw("Delete "),
        Span::styled(
            format!("\"{name}\""),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("?"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(question, rows[1]);

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(
            "[y] Yes",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled("[n / Esc] No", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(buttons, rows[3]);
}

