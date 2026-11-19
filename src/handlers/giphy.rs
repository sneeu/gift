use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::{App, AppMode};
use crate::config::Config;
use crate::events::AppEvent;
use crate::giphy;

pub fn handle_search(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Esc => app.return_to_list(),
        KeyCode::Enter => submit_search(app, config, tx),
        _ => {
            app.giphy.query.handle_key(key);
        }
    }
}

pub fn handle_results(app: &mut App, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
    match key.code {
        KeyCode::Esc => app.return_to_list(),
        KeyCode::Char('/') => app.mode = AppMode::GiphySearch,
        KeyCode::Char('j') | KeyCode::Down => {
            app.giphy_move_down();
            show_selected(app, tx);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.giphy_move_up();
            show_selected(app, tx);
        }
        KeyCode::Enter | KeyCode::Char('u') => {
            if let Some(gif) = app.giphy.selected_gif().cloned() {
                app.enter_upload_from_giphy(&gif);
            }
        }
        KeyCode::Char('o') => {
            if let Some(url) = app.giphy.selected_gif().map(|g| g.original_url.clone()) {
                if let Err(e) = open::that(&url) {
                    app.set_status(format!("Cannot open browser: {e}"), true);
                }
            }
        }
        _ => {}
    }
}

/// Apply a finished search, ignoring results for a query the user has since changed.
pub fn apply_result(
    app: &mut App,
    query: String,
    result: anyhow::Result<Vec<giphy::GiphyGif>>,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    let in_giphy = matches!(app.mode, AppMode::GiphySearch | AppMode::GiphyResults);
    if !in_giphy || query != app.giphy.query.value().trim() {
        return;
    }
    app.giphy.is_searching = false;
    match result {
        Ok(gifs) => {
            if gifs.is_empty() {
                app.set_status(format!("No Giphy results for '{query}'"), true);
            }
            app.giphy.results = gifs;
            app.giphy.selected = 0;
            app.preview_frames.clear();
            show_selected(app, tx);
        }
        Err(e) => app.set_status(format!("Giphy search failed: {e:#}"), true),
    }
}

fn submit_search(app: &mut App, config: &Config, tx: &mpsc::UnboundedSender<AppEvent>) {
    let query = app.giphy.query.value().trim().to_owned();
    if query.is_empty() {
        return;
    }
    app.mode = AppMode::GiphyResults;
    app.giphy.is_searching = true;
    app.clear_status();

    let api_key = config.giphy_api_key.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let result = giphy::search(&api_key, &query).await;
        let _ = tx.send(AppEvent::GiphyResult { query, result });
    });
}

fn show_selected(app: &mut App, tx: &mpsc::UnboundedSender<AppEvent>) {
    if let Some(gif) = app.giphy.selected_gif() {
        let (key, url) = (gif.cache_key(), gif.preview_url.clone());
        super::common::show_preview(app, tx, key, url);
    }
}
