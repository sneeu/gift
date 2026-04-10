use anyhow::{Context, Result};
use bytes::Bytes;
use futures::StreamExt;
use image::DynamicImage;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::io::Read;
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

/// Bridges an async download stream to the sync `gif` decoder.
/// The async side sends `Bytes` chunks; the decoder reads them via `std::io::Read`.
/// Closing the sender signals EOF.
struct ChunkReader {
    rx: std::sync::mpsc::Receiver<Bytes>,
    current: Bytes,
    pos: usize,
}

impl ChunkReader {
    fn new(rx: std::sync::mpsc::Receiver<Bytes>) -> Self {
        Self { rx, current: Bytes::new(), pos: 0 }
    }
}

impl Read for ChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.pos < self.current.len() {
                let n = buf.len().min(self.current.len() - self.pos);
                buf[..n].copy_from_slice(&self.current[self.pos..self.pos + n]);
                self.pos += n;
                return Ok(n);
            }
            match self.rx.recv() {
                Ok(chunk) => { self.current = chunk; self.pos = 0; }
                Err(_) => return Ok(0), // channel closed = EOF
            }
        }
    }
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
    decode_gif_from_reader(std::io::Cursor::new(data))
}

/// Streaming variant: decodes from any `Read` source, sending each composited frame via `tx`
/// as it is decoded. Also returns all frames so the caller can save them to the disk cache.
fn decode_gif_streaming(
    reader: impl Read,
    generation: u64,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<Vec<DynamicImage>> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(reader).context("Failed to read GIF header")?;

    let gif_w = decoder.width() as u32;
    let gif_h = decoder.height() as u32;
    let mut canvas = image::RgbaImage::new(gif_w, gif_h);
    let mut all_frames = Vec::new();

    while let Some(frame) = decoder.read_next_frame().context("Failed to read GIF frame")? {
        let fl = frame.left as u32;
        let ft = frame.top as u32;
        let fw = frame.width as u32;
        let fh = frame.height as u32;

        let pre_composite = canvas.clone();

        for y in 0..fh {
            for x in 0..fw {
                let i = ((y * fw + x) * 4) as usize;
                let a = frame.buffer[i + 3];
                if a == 0 {
                    continue;
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

        let img = DynamicImage::ImageRgba8(canvas.clone());
        all_frames.push(img.clone());
        let _ = tx.send(AppEvent::PreviewFrame { generation, frame: img });

        match frame.dispose {
            gif::DisposalMethod::Background => {
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
                canvas = pre_composite;
            }
            _ => {}
        }
    }

    if all_frames.is_empty() {
        anyhow::bail!("GIF has no frames");
    }
    Ok(all_frames)
}

fn decode_gif_from_reader(reader: impl Read) -> Result<Vec<DynamicImage>> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(reader).context("Failed to read GIF header")?;

    let gif_w = decoder.width() as u32;
    let gif_h = decoder.height() as u32;
    let mut canvas = image::RgbaImage::new(gif_w, gif_h);
    let mut frames = Vec::new();

    while let Some(frame) = decoder.read_next_frame().context("Failed to read GIF frame")? {
        let fl = frame.left as u32;
        let ft = frame.top as u32;
        let fw = frame.width as u32;
        let fh = frame.height as u32;

        let pre_composite = canvas.clone();

        for y in 0..fh {
            for x in 0..fw {
                let i = ((y * fw + x) * 4) as usize;
                let a = frame.buffer[i + 3];
                if a == 0 {
                    continue;
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

        match frame.dispose {
            gif::DisposalMethod::Background => {
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
                canvas = pre_composite;
            }
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

/// Spawn a background task that streams and decodes GIF frames, sending each to the UI
/// as soon as it is decoded rather than waiting for the full download to complete.
pub fn spawn_preview(
    key: String,
    base_url: String,
    generation: u64,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        if let Err(e) = stream_frames(&key, &base_url, generation, &tx).await {
            let _ = tx.send(AppEvent::PreviewError {
                generation,
                message: format!("Preview failed: {e:#}"),
            });
        }
    });
}

async fn stream_frames(
    key: &str,
    base_url: &str,
    generation: u64,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<()> {
    // Check disk cache first — serve all frames at once if available.
    if let Some(frames) = load_cached_frames(key).await {
        let _ = tx.send(AppEvent::PreviewReady { generation, key: key.to_owned(), frames });
        return Ok(());
    }

    let url = format!("{}/{}", base_url.trim_end_matches('/'), key);
    let response = reqwest::get(&url)
        .await
        .with_context(|| format!("Failed to download preview from {url}"))?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {} fetching preview from {url}", response.status());
    }

    // Bridge: async download chunks → sync GIF decoder.
    let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<Bytes>();

    // Decode runs on a blocking thread, reading from ChunkReader as chunks arrive.
    let tx_for_decode = tx.clone();
    let decode_handle = tokio::task::spawn_blocking(move || {
        decode_gif_streaming(ChunkReader::new(chunk_rx), generation, &tx_for_decode)
    });

    // Stream download chunks into the channel.
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Download stream error")?;
        if chunk_tx.send(chunk).is_err() {
            break; // decoder closed its receiver (e.g. invalid GIF header)
        }
    }
    drop(chunk_tx); // closing the channel signals EOF to the decoder

    let frames = decode_handle.await.context("decode task panicked")??;

    let _ = tx.send(AppEvent::PreviewComplete { generation, key: key.to_owned() });

    // Save to disk cache in the background — don't block the caller.
    let key_owned = key.to_owned();
    tokio::spawn(async move {
        let _ = save_frames(&key_owned, &frames).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dir_encodes_key() {
        let dir = frame_dir("foo/bar baz.gif");
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.starts_with("/tmp/gift/previews/"));
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
