use tokio::sync::mpsc;

use crate::app::App;
use crate::config::Config;
use crate::events::AppEvent;

/// Show frames for `key`, from memory if cached, otherwise by fetching `url`.
pub fn show_preview(app: &mut App, tx: &mpsc::UnboundedSender<AppEvent>, key: String, url: String) {
    if !app.load_preview_cached(&key) {
        crate::preview::spawn_preview(key, url, app.preview_generation, tx.clone());
    }
}

/// Show the S3 object `key` in the preview pane.
pub fn show_s3_preview(
    app: &mut App,
    config: &Config,
    tx: &mpsc::UnboundedSender<AppEvent>,
    key: String,
) {
    let url = app.public_url(&key, &config.base_url);
    show_preview(app, tx, key, url);
}

pub fn copy_url(app: &mut App, config: &Config, key: &str) {
    let url = app.public_url(key, &config.base_url);
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&url)) {
        Ok(_) => app.set_status(format!("Copied: {url}"), false),
        Err(e) => app.set_status(format!("Clipboard error: {e}"), true),
    }
}

pub fn open_in_browser(app: &mut App, config: &Config, key: &str) {
    let url = app.public_url(key, &config.base_url);
    if let Err(e) = open::that(&url) {
        app.set_status(format!("Cannot open browser: {e}"), true);
    }
}
