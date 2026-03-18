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
        KeyCode::Esc => app.return_from_preview(),
        KeyCode::Char('j') | KeyCode::Down => {
            app.preview_move_down();
            load_preview(app, config, tx);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.preview_move_up();
            load_preview(app, config, tx);
        }
        KeyCode::Char('c') => {
            if let Some(item) = app.active_item() {
                let key = item.key.clone();
                super::common::copy_url(app, config, &key);
            }
        }
        KeyCode::Char('o') => {
            if let Some(item) = app.active_item() {
                let key = item.key.clone();
                super::common::open_in_browser(app, config, &key);
            }
        }
        KeyCode::Char('n') => {
            if !app.items.is_empty() {
                app.enter_rename();
            }
        }
        _ => {}
    }
}

fn load_preview(app: &mut App, config: &Config, tx: &mpsc::UnboundedSender<AppEvent>) {
    let key_str = app.selected_item().map(|i| i.key.clone()).unwrap_or_default();
    if !app.load_preview_cached(&key_str) {
        crate::preview::spawn_preview(
            key_str,
            config.base_url.clone(),
            app.preview_generation,
            tx.clone(),
        );
    }
}

