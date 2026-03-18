use image::DynamicImage;

use crate::app::GifItem;

/// Results sent from background tasks back to the event loop.
#[derive(Debug)]
pub enum AppEvent {
    /// S3 listing completed.
    ListResult(anyhow::Result<Vec<GifItem>>),
    /// Upload completed with the public URL.
    UploadComplete(anyhow::Result<String>),
    /// Delete completed.
    DeleteComplete(anyhow::Result<()>),
    /// Rename completed with the new key.
    RenameComplete(anyhow::Result<String>),
    /// GIF preview frames decoded and ready.
    /// Carries the generation counter and key so stale results can be discarded
    /// and frames can be cached.
    PreviewReady { generation: u64, key: String, frames: Vec<DynamicImage> },
    /// Preview failed to load (network error, corrupt GIF, etc.).
    /// Only shown if the generation still matches so stale errors don't clobber
    /// a successful load that came in later.
    PreviewError { generation: u64, message: String },
}
