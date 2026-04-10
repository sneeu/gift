use anyhow::{Context, Result};
use image::DynamicImage;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::events::AppEvent;

fn frame_dir(key: &str) -> PathBuf {
    let encoded = utf8_percent_encode(key, NON_ALPHANUMERIC).to_string();
    PathBuf::from("/tmp/gift/previews").join(encoded)
}

fn frame_path(dir: &PathBuf, index: usize) -> PathBuf {
    dir.join(format!("{:04}.png", index))
}

/// Try to load cached frames from disk. Returns `None` if the cache is empty or unreadable.
async fn load_cached_frames(key: &str) -> Option<Vec<DynamicImage>> {
    let dir = frame_dir(key);
    let mut entries = tokio::fs::read_dir(&dir).await.ok()?;
    let mut paths: Vec<PathBuf> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("png") {
            paths.push(p);
        }
    }
    if paths.is_empty() {
        return None;
    }
    paths.sort();
    let mut frames = Vec::with_capacity(paths.len());
    for path in &paths {
        let bytes = tokio::fs::read(path).await.ok()?;
        let img = image::load_from_memory(&bytes).ok()?;
        frames.push(img);
    }
    Some(frames)
}

/// Decode a GIF into fully-composited frames.
///
/// GIF frames are deltas: each frame is positioned at (left, top) within the full canvas
/// and may be smaller than the canvas.  Transparent pixels mean "keep whatever was there".
/// We maintain a canvas and composite each frame onto it, then apply the disposal method
/// to prepare for the next frame.
pub fn decode_gif(data: &[u8]) -> Result<Vec<DynamicImage>> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(std::io::Cursor::new(data))
        .context("Failed to read GIF header")?;

    let gif_w = decoder.width() as u32;
    let gif_h = decoder.height() as u32;

    let mut canvas = image::RgbaImage::new(gif_w, gif_h);
    let mut frames = Vec::new();

    while let Some(frame) = decoder.read_next_frame().context("Failed to read GIF frame")? {
        let fl = frame.left as u32;
        let ft = frame.top as u32;
        let fw = frame.width as u32;
        let fh = frame.height as u32;

        // Snapshot canvas before compositing — needed for DisposalMethod::Previous.
        let pre_composite = canvas.clone();

        // Composite this frame's pixels onto the canvas.
        // With RGBA output each pixel is 4 bytes; alpha == 0 means transparent ("leave as is").
        for y in 0..fh {
            for x in 0..fw {
                let i = ((y * fw + x) * 4) as usize;
                let a = frame.buffer[i + 3];
                if a == 0 {
                    continue; // transparent — keep canvas pixel
                }
                let cx = fl + x;
                let cy = ft + y;
                if cx < gif_w && cy < gif_h {
                    canvas.put_pixel(
                        cx,
                        cy,
                        image::Rgba([frame.buffer[i], frame.buffer[i + 1], frame.buffer[i + 2], a]),
                    );
                }
            }
        }

        frames.push(DynamicImage::ImageRgba8(canvas.clone()));

        // Apply disposal method to set up the canvas for the next frame.
        match frame.dispose {
            gif::DisposalMethod::Background => {
                // Clear this frame's area to transparent.
                for y in 0..fh {
                    for x in 0..fw {
                        let cx = fl + x;
                        let cy = ft + y;
                        if cx < gif_w && cy < gif_h {
                            canvas.put_pixel(cx, cy, image::Rgba([0, 0, 0, 0]));
                        }
                    }
                }
            }
            gif::DisposalMethod::Previous => {
                // Restore to the canvas state before this frame was composited.
                canvas = pre_composite;
            }
            // Keep / Any: leave canvas as-is.
            _ => {}
        }
    }

    if frames.is_empty() {
        anyhow::bail!("GIF has no frames");
    }
    Ok(frames)
}

/// Save decoded frames to the disk cache.
async fn save_frames(key: &str, frames: &[DynamicImage]) -> Result<()> {
    let dir = frame_dir(key);
    tokio::fs::create_dir_all(&dir).await?;
    for (i, frame) in frames.iter().enumerate() {
        let path = frame_path(&dir, i);
        let mut buf = Vec::new();
        frame
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .with_context(|| format!("Failed to encode frame {i} as PNG"))?;
        tokio::fs::write(&path, buf).await?;
    }
    Ok(())
}

/// Spawn a background task that loads or fetches GIF frames and sends them via the channel.
pub fn spawn_preview(
    key: String,
    base_url: String,
    generation: u64,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let result = load_frames(&key, &base_url).await;
        match result {
            Ok(frames) => {
                let _ = tx.send(AppEvent::PreviewReady { generation, key, frames });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::PreviewError {
                    generation,
                    message: format!("Preview failed: {e:#}"),
                });
            }
        }
    });
}

async fn load_frames(key: &str, base_url: &str) -> Result<Vec<DynamicImage>> {
    let t0 = std::time::Instant::now();

    // Check disk cache first
    eprintln!("[{:.3}s] {key}: checking disk cache", t0.elapsed().as_secs_f32());
    if let Some(frames) = load_cached_frames(key).await {
        eprintln!("[{:.3}s] {key}: disk cache hit ({} frames)", t0.elapsed().as_secs_f32(), frames.len());
        return Ok(frames);
    }
    eprintln!("[{:.3}s] {key}: disk cache miss, starting download", t0.elapsed().as_secs_f32());

    // Fetch from CDN
    let url = format!("{}/{}", base_url.trim_end_matches('/'), key);
    let response = reqwest::get(&url)
        .await
        .with_context(|| format!("Failed to download preview from {url}"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} fetching preview from {url}");
    }
    let data = response.bytes().await.context("Failed to read preview response")?;
    eprintln!("[{:.3}s] {key}: download complete ({} bytes)", t0.elapsed().as_secs_f32(), data.len());

    // decode_gif is CPU-bound (decompresses every frame); run it off the async executor
    // so it doesn't block a tokio worker thread that the event loop depends on.
    eprintln!("[{:.3}s] {key}: starting GIF decode", t0.elapsed().as_secs_f32());
    let frames = tokio::task::spawn_blocking(move || decode_gif(&data))
        .await
        .context("decode_gif task panicked")??;
    eprintln!("[{:.3}s] {key}: GIF decode complete ({} frames)", t0.elapsed().as_secs_f32(), frames.len());

    // Save to disk cache in the background so the caller gets frames immediately.
    let key_owned = key.to_owned();
    let frames_for_cache = frames.clone();
    tokio::spawn(async move {
        eprintln!("[background] {key_owned}: starting cache save");
        let _ = save_frames(&key_owned, &frames_for_cache).await;
        eprintln!("[background] {key_owned}: cache save complete");
    });

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dir_encodes_key() {
        let dir = frame_dir("foo/bar baz.gif");
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.starts_with("/tmp/gift/previews/"));
        // Should not contain raw slashes or spaces in the encoded segment
        let segment = dir_str.strip_prefix("/tmp/gift/previews/").unwrap();
        assert!(!segment.contains('/'));
        assert!(!segment.contains(' '));
    }

    #[test]
    fn frame_dir_different_keys_produce_different_dirs() {
        assert_ne!(frame_dir("a.gif"), frame_dir("b.gif"));
    }

    #[test]
    fn decode_gif_rejects_invalid_data() {
        let result = decode_gif(b"not a gif");
        assert!(result.is_err());
    }
}
