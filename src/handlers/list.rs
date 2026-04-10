use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::{App, SortField};
use crate::cache;
use crate::config::Config;
use crate::events::AppEvent;
use crate::s3;

pub fn handle(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Char('q') => {} // handled as quit in run()
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
            app.clear_status();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
            app.clear_status();
        }
        KeyCode::Enter => {
            if !app.items.is_empty() {
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
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('u') => app.enter_upload(),
        KeyCode::Char('n') => {
            if !app.items.is_empty() {
                app.enter_rename();
            }
        }
        KeyCode::Char('d') => {
            if !app.items.is_empty() {
                app.enter_delete();
            }
        }
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                let key = item.key.clone();
                super::common::copy_url(app, config, &key);
            }
        }
        KeyCode::Char('o') => {
            if let Some(item) = app.selected_item() {
                let key = item.key.clone();
                super::common::open_in_browser(app, config, &key);
            }
        }
        KeyCode::Char('f') => { app.sort_order.toggle_to(SortField::Name); app.sort_items(); }
        KeyCode::Char('s') => { app.sort_order.toggle_to(SortField::Size); app.sort_items(); }
        KeyCode::Char('t') => { app.sort_order.toggle_to(SortField::Date); app.sort_items(); }
        KeyCode::Char('r') => spawn_list(app, config, client, tx, true),
        KeyCode::Char('?') => app.enter_help(),
        _ => {}
    }
}

pub fn spawn_list(
    app: &mut App,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
    force: bool,
) {
    app.is_loading = true;
    if force {
        // Don't clear status — the caller may have just set a success message.
    } else {
        app.clear_status();
    }
    let client = client.clone();
    let bucket = config.bucket.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        if force {
            cache::invalidate_listing().await;
        } else if let Some(items) = cache::load_listing().await {
            let _ = tx.send(AppEvent::ListResult(Ok(items)));
            return;
        }
        let result = s3::list_all(&client, &bucket).await;
        if let Ok(ref items) = result {
            let _ = cache::save_listing(items).await;
        }
        let _ = tx.send(AppEvent::ListResult(result));
    });
}

