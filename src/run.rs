use std::io::{self, Write};
use std::time::Duration;

use arboard;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::time;

use aws_sdk_s3::Client;

use crate::app::{App, AppMode};
use crate::config::Config;
use crate::events::AppEvent;
use crate::handlers;
use crate::s3;
use crate::ui;

const TICK_MS: u64 = 100;

pub async fn run(config: Config) -> anyhow::Result<()> {
    // Set up terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide,
        crossterm::event::EnableBracketedPaste,
    )?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let client = s3::build_client(&config).await;
    let result = event_loop(&mut terminal, config, client).await;

    // Restore terminal regardless of result
    crossterm::terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        crossterm::event::DisableBracketedPaste,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    )?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
    client: Client,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let mut app = App::new();
    let mut ticker = time::interval(Duration::from_millis(TICK_MS));
    let mut events = EventStream::new();

    // Kick off initial list load
    handlers::list::spawn_list(&mut app, &config, &client, &tx, false);

    loop {
        // Clear if needed before drawing
        if app.needs_clear {
            terminal.clear()?;
            app.needs_clear = false;
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // After draw, render GIF frame via viuer if in Preview with frames ready
        render_viuer_frame(&app)?;

        tokio::select! {
            // Keyboard / terminal events
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        if should_quit(&key) {
                            break;
                        }
                        dispatch_key(&mut app, key, &config, &client, &tx);
                    }
                    Some(Ok(Event::Paste(text))) => {
                        dispatch_paste(&mut app, &text);
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        app.needs_clear = true;
                    }
                    Some(Err(e)) => {
                        eprintln!("Input error: {e}");
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }

            // Background task results
            maybe_app_event = rx.recv() => {
                if let Some(event) = maybe_app_event {
                    handle_app_event(&mut app, event, &config, &client, &tx);
                }
            }

            // Tick: advance spinner + animation frame
            _ = ticker.tick() => {
                app.spinner_tick = app.spinner_tick.wrapping_add(1);
                app.advance_frame();
                app.tick_status();
            }
        }
    }

    Ok(())
}

fn should_quit(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL)
    )
}

fn dispatch_key(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    if should_quit(&key) {
        return;
    }
    match app.mode {
        AppMode::List => handlers::list::handle(app, key, config, client, tx),
        AppMode::Search => handlers::search::handle(app, key, config, client, tx),
        AppMode::Preview => handlers::preview::handle(app, key, config, client, tx),
        AppMode::UploadForm => handlers::upload::handle(app, key, config, client, tx),
        AppMode::RenameForm => handlers::rename::handle(app, key, config, client, tx),
        AppMode::DeleteConfirm => handlers::delete::handle(app, key, config, client, tx),
        AppMode::Help => handlers::help::handle(app, key),
    }
}

fn dispatch_paste(app: &mut App, text: &str) {
    match app.mode {
        AppMode::Search => {
            app.search_input.insert_str(text);
        }
        AppMode::UploadForm => match app.upload_form.focused {
            crate::app::UploadField::Source => app.upload_form.source.insert_str(text),
            crate::app::UploadField::Name => app.upload_form.name.insert_str(text),
        },
        AppMode::RenameForm => {
            app.rename_form.to.insert_str(text);
        }
        _ => {}
    }
}

fn handle_app_event(
    app: &mut App,
    event: AppEvent,
    config: &Config,
    client: &Client,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    match event {
        AppEvent::ListResult(Ok(items)) => {
            app.items = items;
            app.sort_items();
            app.is_loading = false;
            // Clamp selected to valid range
            if app.selected >= app.items.len() && !app.items.is_empty() {
                app.selected = app.items.len() - 1;
            }
        }
        AppEvent::ListResult(Err(e)) => {
            app.is_loading = false;
            app.set_status(format!("Error loading GIFs: {e:#}"), true);
        }
        AppEvent::UploadComplete(Ok(url)) => {
            app.is_loading = false;
            handlers::list::spawn_list(app, config, client, tx, true);
            // Copy URL to clipboard
            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&url)) {
                Ok(_) => app.set_status(format!("Uploaded & copied: {url}"), false),
                Err(_) => app.set_status(format!("Uploaded: {url}"), false),
            }
        }
        AppEvent::UploadComplete(Err(e)) => {
            app.is_loading = false;
            app.set_status(format!("Upload failed: {e}"), true);
        }
        AppEvent::DeleteComplete(Ok(())) => {
            app.is_loading = false;
            handlers::list::spawn_list(app, config, client, tx, true);
            app.set_status("Deleted", false);
        }
        AppEvent::DeleteComplete(Err(e)) => {
            app.is_loading = false;
            app.set_status(format!("Delete failed: {e}"), true);
        }
        AppEvent::RenameComplete(Ok(new_key)) => {
            app.is_loading = false;
            handlers::list::spawn_list(app, config, client, tx, true);
            app.set_status(format!("Renamed to {new_key}"), false);
        }
        AppEvent::RenameComplete(Err(e)) => {
            app.is_loading = false;
            app.set_status(format!("Rename failed: {e}"), true);
        }
        AppEvent::PreviewError { generation, message } => {
            if generation == app.preview_generation {
                app.set_status(message, true);
            }
        }
        AppEvent::PreviewReady { generation, key, frames } => {
            // Always cache — the decoded frames are valid regardless of whether we still
            // need them for the current selection.
            app.preview_cache.insert(key, frames.clone());
            // Only apply to the display if this result is still current.
            if generation == app.preview_generation {
                app.preview_frames = frames;
                app.preview_frame_index = 0;
            }
        }
    }
}

fn render_viuer_frame(app: &App) -> anyhow::Result<()> {
    if app.mode != AppMode::Preview {
        return Ok(());
    }
    let Some(frame) = app.preview_frames.get(app.preview_frame_index) else {
        return Ok(());
    };
    let Some(rect) = app.preview_pane_rect else {
        return Ok(());
    };

    // Inner area of the bordered pane (subtract 1 on each side)
    let inner_x = rect.x + 1;
    let inner_y = rect.y + 1;
    let inner_w = rect.width.saturating_sub(2);
    let inner_h = rect.height.saturating_sub(2);

    if inner_w == 0 || inner_h == 0 {
        return Ok(());
    }

    // Move cursor to the inner top-left of the preview pane
    execute!(
        io::stdout(),
        crossterm::cursor::MoveTo(inner_x, inner_y),
    )?;

    let conf = viuer::Config {
        absolute_offset: true,
        x: inner_x,
        y: inner_y as i16,
        width: Some(inner_w as u32),
        height: Some(inner_h as u32),
        // Kitty and iTerm protocols send an escape sequence then read stdin waiting for
        // the terminal's acknowledgment.  crossterm's EventStream owns stdin, so that
        // read never unblocks — freezing the event loop.  Use sixel / half-block chars
        // instead; neither requires a round-trip.
        use_kitty: false,
        use_iterm: false,
        ..Default::default()
    };

    viuer::print(frame, &conf)
        .map_err(|e| anyhow::anyhow!("viuer render error: {e}"))?;

    io::stdout().flush()?;
    Ok(())
}
