use ratatui::{
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use crate::app::{App, AppMode};

use super::{preview, status_bar};

pub fn draw(f: &mut Frame, app: &mut App) {
    let (left_area, right_area, status_area) = preview::split(f.area());

    let title = if app.giphy.is_searching {
        format!(" Giphy {} ", app.spinner_char())
    } else {
        format!(" Giphy ({}) ", app.giphy.results.len())
    };
    let items: Vec<ListItem> = app
        .giphy
        .results
        .iter()
        .map(|g| ListItem::new(Line::from(Span::raw(g.title.clone()))))
        .collect();

    let mut state = ListState::default();
    if !app.giphy.results.is_empty() {
        state.select(Some(app.giphy.selected));
    }
    f.render_stateful_widget(preview::list(items, title), left_area, &mut state);

    let (pane_title, empty_text) = match app.giphy.selected_gif() {
        Some(g) => (format!(" {} ", g.title), "Loading…"),
        None => (" Preview ".to_owned(), "Type a query and press Enter"),
    };
    preview::draw_pane(f, app, right_area, pane_title, empty_text);

    match app.mode {
        AppMode::GiphySearch => {
            status_bar::draw_input(f, app, status_area, "giphy: ", &app.giphy.query)
        }
        _ => status_bar::draw_giphy_results(f, app, status_area),
    }
}
