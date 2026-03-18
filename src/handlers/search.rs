use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::sync::mpsc;

use crate::app::App;
use crate::config::Config;
use crate::events::AppEvent;
use crate::search::fuzzy_search;

pub fn handle(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    _client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Esc => app.exit_search(),
        KeyCode::Enter => {
            if !app.search_results.is_empty() {
                let list_idx =
                    app.search_results[app.search_selected.min(app.search_results.len() - 1)];
                app.selected = list_idx;
                let key_str = app.selected_item().map(|i| i.key.clone()).unwrap_or_default();
                app.enter_preview();
                if !app.load_preview_cached(&key_str) {
                    crate::preview::spawn_preview(
                        key_str,
                        config.base_url.clone(),
                        app.preview_generation,
                        tx.clone(),
                    );
                }
            }
        }
        KeyCode::Up => app.search_move_up(),
        KeyCode::Down => app.search_move_down(),
        _ => {
            if app.search_input.handle_key(key) {
                update_results(app);
            }
        }
    }
}

fn update_results(app: &mut App) {
    let matcher = SkimMatcherV2::default();
    let query = app.search_input.value().to_owned();
    let results = fuzzy_search(&matcher, &query, &app.items);
    app.search_results = results.into_iter().map(|m| m.index).collect();
    app.search_selected = 0;
}
