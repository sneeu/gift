use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::App;
use crate::config::Config;
use crate::download;
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
        KeyCode::Esc => app.return_to_list(),
        KeyCode::Enter => submit_rename(app, config, client, tx),
        _ => {
            app.rename_form.to.handle_key(key);
        }
    }
}

fn submit_rename(
    app: &mut App,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    let old_key = app.rename_form.from.clone();
    let raw = app.rename_form.to.value().trim().to_owned();

    if raw.is_empty() {
        app.set_status("New name is required", true);
        return;
    }

    let new_key = download::ensure_gif_extension(&raw);
    if new_key == old_key {
        app.set_status("Name unchanged", true);
        return;
    }

    let bucket = config.bucket.clone();
    let client = client.clone();
    let tx = tx.clone();

    app.return_to_list();
    app.is_loading = true;

    tokio::spawn(async move {
        let result = s3::rename(&client, &bucket, &old_key, &new_key)
            .await
            .map(|_| new_key);
        let _ = tx.send(AppEvent::RenameComplete(result));
    });
}
