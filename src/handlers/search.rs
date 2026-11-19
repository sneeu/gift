use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::App;
use crate::config::Config;
use crate::events::AppEvent;

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
                super::common::show_s3_preview(app, config, tx, key_str);
            }
        }
        KeyCode::Up => app.search_move_up(),
        KeyCode::Down => app.search_move_down(),
        _ => {
            if app.search_input.handle_key(key) {
                app.search_selected = 0;
                app.update_search_results();
            }
        }
    }
}

