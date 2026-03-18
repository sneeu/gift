use super::util::popup_area;
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, _app: &App) {
    let area = popup_area(f.area(), 50, 27);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = help_lines();
    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_owned(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn binding(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {key:<18}"),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_owned(), Style::default().fg(Color::Gray)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn help_lines() -> Vec<Line<'static>> {
    vec![
        blank(),
        section("List"),
        binding("j / ↓", "Move down"),
        binding("k / ↑", "Move up"),
        binding("Enter", "Preview selected"),
        binding("/", "Search"),
        binding("u", "Upload"),
        binding("n", "Rename"),
        binding("d", "Delete"),
        binding("c", "Copy URL"),
        binding("o", "Open in browser"),
        binding("r", "Refresh list"),
        binding("q / Ctrl+C", "Quit"),
        blank(),
        section("Preview"),
        binding("Esc", "Back to list"),
        binding("j / ↓", "Next GIF"),
        binding("k / ↑", "Previous GIF"),
        binding("c", "Copy URL"),
        binding("o", "Open in browser"),
        binding("n", "Rename"),
        blank(),
        section("Upload Form"),
        binding("Tab", "Switch field"),
        binding("Enter", "Submit"),
        binding("Esc", "Cancel"),
        blank(),
        binding("? / Esc", "Close help"),
    ]
}

