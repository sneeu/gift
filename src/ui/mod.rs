use ratatui::Frame;

use crate::app::{App, AppMode};

mod delete;
mod help;
mod list;
mod preview;
mod rename;
mod search;
mod status_bar;
mod upload;
mod util;

/// Top-level draw dispatcher. Renders the appropriate screen based on `app.mode`.
pub fn draw(f: &mut Frame, app: &mut App) {
    match app.mode {
        AppMode::List => {
            list::draw(f, app);
        }
        AppMode::Search => {
            search::draw(f, app);
        }
        AppMode::Preview => {
            preview::draw(f, app);
        }
        AppMode::UploadForm => {
            list::draw(f, app);
            upload::draw(f, app);
        }
        AppMode::RenameForm => {
            list::draw(f, app);
            rename::draw(f, app);
        }
        AppMode::DeleteConfirm => {
            list::draw(f, app);
            delete::draw(f, app);
        }
        AppMode::Help => {
            list::draw(f, app);
            help::draw(f, app);
        }
    }
}
