use crate::widgets::text_input::TextInput;
use image::DynamicImage;
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GifItem {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    List,
    Search,
    Preview,
    UploadForm,
    RenameForm,
    DeleteConfirm,
    Help,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum UploadField {
    #[default]
    Source,
    Name,
}

#[derive(Debug, Default)]
pub struct UploadForm {
    pub source: TextInput,
    pub name: TextInput,
    pub focused: UploadField,
    pub confirm_overwrite: bool,
}

#[derive(Debug, Default)]
pub struct RenameForm {
    pub from: String,
    pub to: TextInput,
}

pub struct App {
    pub mode: AppMode,
    /// Mode we came from — so Preview knows whether Esc returns to List or Search.
    pub prev_mode: AppMode,
    pub items: Vec<GifItem>,
    pub selected: usize,
    pub status_message: Option<String>,
    pub status_is_error: bool,
    pub status_expires_at: Option<Instant>,
    pub is_loading: bool,
    pub upload_form: UploadForm,
    pub rename_form: RenameForm,
    pub preview_frames: Vec<DynamicImage>,
    pub preview_frame_index: usize,
    pub spinner_tick: usize,
    pub needs_clear: bool,
    pub search_input: TextInput,
    pub search_results: Vec<usize>,
    pub search_selected: usize,
    /// Rect of the right pane in Preview mode — set during draw, read post-draw for viuer.
    pub preview_pane_rect: Option<Rect>,
    /// Incremented every time we start loading a new preview.  Tasks include the generation
    /// at spawn time; stale results are ignored if the counter has moved on.
    pub preview_generation: u64,
    /// In-memory cache of decoded frames keyed by S3 object key.
    /// Populated when a PreviewReady event is received; makes revisiting GIFs instant.
    pub preview_cache: HashMap<String, Vec<DynamicImage>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::List,
            prev_mode: AppMode::List,
            items: Vec::new(),
            selected: 0,
            status_message: None,
            status_is_error: false,
            status_expires_at: None,
            is_loading: true,
            upload_form: UploadForm::default(),
            rename_form: RenameForm::default(),
            preview_frames: Vec::new(),
            preview_frame_index: 0,
            spinner_tick: 0,
            needs_clear: false,
            search_input: TextInput::default(),
            search_results: Vec::new(),
            search_selected: 0,
            preview_pane_rect: None,
            preview_generation: 0,
            preview_cache: HashMap::new(),
        }
    }

    // --- Item accessors ---

    pub fn selected_item(&self) -> Option<&GifItem> {
        self.items.get(self.selected)
    }

    pub fn search_selected_item(&self) -> Option<&GifItem> {
        self.search_results
            .get(self.search_selected)
            .and_then(|&i| self.items.get(i))
    }

    /// The contextually active item (search-aware).
    pub fn active_item(&self) -> Option<&GifItem> {
        match self.prev_mode {
            AppMode::Search => self.search_selected_item(),
            _ => self.selected_item(),
        }
    }

    // --- State transitions ---

    pub fn enter_preview(&mut self) {
        self.prev_mode = self.mode.clone();
        self.mode = AppMode::Preview;
        self.preview_frames.clear();
        self.preview_frame_index = 0;
        self.preview_generation = self.preview_generation.wrapping_add(1);
        self.needs_clear = true;
    }

    /// Load frames for `key` from the in-memory cache if available, otherwise return `false`
    /// so the caller knows to spawn a fetch task.  Always bumps the generation so any
    /// in-flight task for the previous GIF is ignored when it eventually arrives.
    pub fn load_preview_cached(&mut self, key: &str) -> bool {
        self.preview_frames.clear();
        self.preview_frame_index = 0;
        self.preview_generation = self.preview_generation.wrapping_add(1);
        if let Some(frames) = self.preview_cache.get(key) {
            self.preview_frames = frames.clone();
            true
        } else {
            false
        }
    }

    pub fn return_from_preview(&mut self) {
        self.mode = self.prev_mode.clone();
        self.preview_frames.clear();
        self.preview_frame_index = 0;
        self.preview_pane_rect = None;
        self.needs_clear = true;
    }

    pub fn enter_search(&mut self) {
        self.mode = AppMode::Search;
        self.search_input.clear();
        self.search_results = (0..self.items.len()).collect();
        self.search_selected = 0;
    }

    pub fn exit_search(&mut self) {
        self.mode = AppMode::List;
        self.search_input.clear();
        self.search_results.clear();
        self.search_selected = 0;
    }

    pub fn enter_upload(&mut self) {
        self.mode = AppMode::UploadForm;
        self.upload_form = UploadForm::default();
    }

    pub fn enter_rename(&mut self) {
        let from = self.active_item().map(|i| i.key.clone()).unwrap_or_default();
        self.mode = AppMode::RenameForm;
        self.rename_form = RenameForm {
            from,
            to: TextInput::default(),
        };
    }

    pub fn enter_delete(&mut self) {
        self.mode = AppMode::DeleteConfirm;
    }

    pub fn enter_help(&mut self) {
        self.mode = AppMode::Help;
    }

    pub fn return_to_list(&mut self) {
        self.mode = AppMode::List;
        self.prev_mode = AppMode::List;
        self.needs_clear = true;
    }

    // --- Status ---

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status_message = Some(msg.into());
        self.status_is_error = is_error;
        // Success messages auto-expire; errors stay until the user navigates away.
        self.status_expires_at = if is_error {
            None
        } else {
            Some(Instant::now() + Duration::from_secs(5))
        };
    }

    /// Called on every tick — clears expired success messages.
    pub fn tick_status(&mut self) {
        if let Some(exp) = self.status_expires_at {
            if Instant::now() >= exp {
                self.clear_status();
            }
        }
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.status_is_error = false;
        self.status_expires_at = None;
    }

    // --- Animation helpers ---

    pub fn spinner_char(&self) -> char {
        const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        FRAMES[self.spinner_tick % FRAMES.len()]
    }

    pub fn advance_frame(&mut self) {
        if !self.preview_frames.is_empty() {
            self.preview_frame_index =
                (self.preview_frame_index + 1) % self.preview_frames.len();
        }
    }

    // --- List navigation ---

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    pub fn search_move_up(&mut self) {
        if self.search_selected > 0 {
            self.search_selected -= 1;
        }
    }

    pub fn search_move_down(&mut self) {
        if self.search_selected + 1 < self.search_results.len() {
            self.search_selected += 1;
        }
    }

    /// Navigate up, respecting search context.
    pub fn preview_move_up(&mut self) {
        if self.prev_mode == AppMode::Search {
            self.search_move_up();
            if let Some(&idx) = self.search_results.get(self.search_selected) {
                self.selected = idx;
            }
        } else {
            self.move_up();
        }
    }

    /// Navigate down, respecting search context.
    pub fn preview_move_down(&mut self) {
        if self.prev_mode == AppMode::Search {
            self.search_move_down();
            if let Some(&idx) = self.search_results.get(self.search_selected) {
                self.selected = idx;
            }
        } else {
            self.move_down();
        }
    }

    /// Public URL for a given key.
    pub fn public_url(&self, key: &str, base_url: &str) -> String {
        format!("{}/{}", base_url.trim_end_matches('/'), key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str) -> GifItem {
        GifItem { key: key.into(), size: 100, last_modified: "2024-01-01".into() }
    }

    #[test]
    fn enter_preview_clears_frames_and_sets_mode() {
        let mut app = App::new();
        app.preview_frames.push(DynamicImage::new_rgb8(1, 1));
        app.preview_frame_index = 3;
        app.enter_preview();
        assert!(app.preview_frames.is_empty());
        assert_eq!(app.preview_frame_index, 0);
        assert_eq!(app.mode, AppMode::Preview);
        assert!(app.needs_clear);
    }

    #[test]
    fn return_from_preview_restores_prev_mode() {
        let mut app = App::new();
        app.mode = AppMode::Search;
        app.enter_preview();
        assert_eq!(app.prev_mode, AppMode::Search);
        app.return_from_preview();
        assert_eq!(app.mode, AppMode::Search);
        assert!(app.preview_frames.is_empty());
        assert!(app.needs_clear);
    }

    #[test]
    fn enter_search_populates_all_results() {
        let mut app = App::new();
        app.items = vec![item("a.gif"), item("b.gif")];
        app.enter_search();
        assert_eq!(app.search_results, vec![0, 1]);
        assert_eq!(app.mode, AppMode::Search);
    }

    #[test]
    fn exit_search_clears_state() {
        let mut app = App::new();
        app.items = vec![item("a.gif")];
        app.enter_search();
        app.exit_search();
        assert_eq!(app.mode, AppMode::List);
        assert!(app.search_results.is_empty());
        assert!(app.search_input.value().is_empty());
    }

    #[test]
    fn spinner_char_is_stable_across_full_cycles() {
        let mut app = App::new();
        let c0 = app.spinner_char();
        app.spinner_tick = 10; // full cycle
        assert_eq!(app.spinner_char(), c0);
    }

    #[test]
    fn advance_frame_wraps() {
        let mut app = App::new();
        app.preview_frames = vec![DynamicImage::new_rgb8(1, 1), DynamicImage::new_rgb8(1, 1)];
        app.preview_frame_index = 1;
        app.advance_frame();
        assert_eq!(app.preview_frame_index, 0);
    }

    #[test]
    fn advance_frame_no_op_when_empty() {
        let mut app = App::new();
        app.advance_frame(); // should not panic
        assert_eq!(app.preview_frame_index, 0);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let mut app = App::new();
        app.selected = 0;
        app.move_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_down_clamps_at_last() {
        let mut app = App::new();
        app.items = vec![item("a.gif")];
        app.selected = 0;
        app.move_down();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn selected_item_returns_correct_item() {
        let mut app = App::new();
        app.items = vec![item("a.gif"), item("b.gif")];
        app.selected = 1;
        assert_eq!(app.selected_item().unwrap().key, "b.gif");
    }

    #[test]
    fn public_url_format() {
        let app = App::new();
        assert_eq!(
            app.public_url("cat.gif", "https://cdn.example.com"),
            "https://cdn.example.com/cat.gif"
        );
        // Trailing slash on base_url should not double-slash
        assert_eq!(
            app.public_url("cat.gif", "https://cdn.example.com/"),
            "https://cdn.example.com/cat.gif"
        );
    }
}
