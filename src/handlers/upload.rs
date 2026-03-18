use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::{App, UploadField};
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
        KeyCode::Tab => {
            app.upload_form.focused = match app.upload_form.focused {
                UploadField::Source => UploadField::Name,
                UploadField::Name => UploadField::Source,
            };
        }
        KeyCode::Enter => submit_upload(app, config, client, tx),
        _ => {
            match app.upload_form.focused {
                UploadField::Source => {
                    if app.upload_form.source.handle_key(key) {
                        app.upload_form.confirm_overwrite = false;
                    }
                }
                UploadField::Name => {
                    if app.upload_form.name.handle_key(key) {
                        app.upload_form.confirm_overwrite = false;
                    }
                }
            };
        }
    }
}

fn submit_upload(
    app: &mut App,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    let source = app.upload_form.source.value().trim().to_owned();
    if source.is_empty() {
        app.set_status("Source is required", true);
        return;
    }

    // For local file paths, validate before spawning.
    if !source.starts_with("http://") && !source.starts_with("https://") {
        if !std::path::Path::new(&source).exists() {
            app.set_status(format!("File not found: {source}"), true);
            return;
        }
    }

    let raw_name = app.upload_form.name.value().trim().to_owned();
    let name = if raw_name.is_empty() {
        download::basename_from_url(&source).unwrap_or_else(|| "upload.gif".to_owned())
    } else {
        raw_name
    };
    let key = download::ensure_gif_extension(&name);

    // Warn on clash; require a second Enter to overwrite.
    if !app.upload_form.confirm_overwrite && app.items.iter().any(|i| i.key == key) {
        app.upload_form.confirm_overwrite = true;
        app.set_status(
            format!("'{key}' already exists — press Enter again to overwrite"),
            true,
        );
        return;
    }

    let bucket = config.bucket.clone();
    let base_url = config.base_url.clone();
    let client = client.clone();
    let tx = tx.clone();

    app.return_to_list();
    app.is_loading = true;
    app.set_status("Uploading…", false);

    tokio::spawn(async move {
        let result = async {
            let data = download::fetch_source(&source).await?;
            s3::upload(&client, &bucket, &key, data).await?;
            Ok::<String, anyhow::Error>(format!("{}/{}", base_url.trim_end_matches('/'), key))
        }
        .await;
        let _ = tx.send(AppEvent::UploadComplete(result));
    });
}
