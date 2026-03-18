use aws_sdk_s3::Client;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::App;
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
        KeyCode::Char('y') => submit_delete(app, config, client, tx),
        KeyCode::Char('n') | KeyCode::Esc => app.return_to_list(),
        _ => {}
    }
}

fn submit_delete(
    app: &mut App,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    let Some(item) = app.active_item() else {
        app.return_to_list();
        return;
    };
    let key = item.key.clone();
    let bucket = config.bucket.clone();
    let client = client.clone();
    let tx = tx.clone();

    app.return_to_list();
    app.is_loading = true;

    tokio::spawn(async move {
        let result = s3::delete(&client, &bucket, &key).await;
        let _ = tx.send(AppEvent::DeleteComplete(result));
    });
}
