use crate::app::App;
use crate::config::Config;

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
