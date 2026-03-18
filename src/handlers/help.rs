use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

pub fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            app.return_to_list();
        }
        _ => {}
    }
}
