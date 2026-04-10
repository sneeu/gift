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
    /// All frames ready at once (disk cache hit path).
    PreviewReady { generation: u64, key: String, frames: Vec<DynamicImage> },
    /// One frame decoded during streaming (download+decode overlap path).
    /// Frames arrive in order; the UI appends each one as it arrives.
    PreviewFrame { generation: u64, frame: DynamicImage },
    /// Streaming decode finished — all PreviewFrame events have been sent.
    PreviewComplete { generation: u64, key: String },
    /// Preview failed to load (network error, corrupt GIF, etc.).
    /// Only shown if the generation still matches so stale errors don't clobber
    /// a successful load that came in later.
    PreviewError { generation: u64, message: String },
}
