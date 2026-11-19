use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::widgets::text_input::TextInput;

const HINT_LIST: &str =
    "j/k Move  Enter Preview  / Search  u Upload  g Giphy  n Rename  d Delete  c Copy URL  f/s/t Sort  r Refresh  ? Help  q Quit";
const HINT_PREVIEW: &str = "Esc Back  j/k Navigate  c Copy URL  o Open  n Rename";
const HINT_GIPHY_RESULTS: &str = "Esc Back  j/k Navigate  Enter Upload  / Edit query  o Open";

pub fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    draw_with_hint(f, app, area, HINT_LIST);
}

pub fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    draw_with_hint(f, app, area, HINT_PREVIEW);
}

pub fn draw_giphy_results(f: &mut Frame, app: &App, area: Rect) {
    draw_with_hint(f, app, area, HINT_GIPHY_RESULTS);
}

pub fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    draw_input(f, app, area, "/", &app.search_input);
}

/// Show `input` after `prefix` with a cursor indicator, unless a status message is set.
pub fn draw_input(f: &mut Frame, app: &App, area: Rect, prefix: &str, input: &TextInput) {
    if let Some(ref msg) = app.status_message {
        let style = if app.status_is_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        let para = Paragraph::new(Line::from(Span::styled(msg, style)));
        f.render_widget(para, area);
        return;
    }

    let line = Line::from(vec![
        Span::raw(prefix.to_owned()),
        Span::raw(input.value()),
        Span::styled("█", Style::default().fg(Color::Gray)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_with_hint(f: &mut Frame, app: &App, area: Rect, hint: &str) {
    let (text, style) = if let Some(ref msg) = app.status_message {
        let style = if app.status_is_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        (msg.as_str(), style)
    } else {
        (hint, Style::default().fg(Color::DarkGray))
    };

    let para = Paragraph::new(Line::from(Span::styled(text, style)));
    f.render_widget(para, area);
}
