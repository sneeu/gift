use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::GifItem;

const TTL_SECS: u64 = 5 * 60;

#[derive(Serialize, Deserialize)]
struct CacheFile {
    written_at: u64,
    items: Vec<GifItem>,
}

fn cache_path() -> PathBuf {
    PathBuf::from("/tmp/gift/listing.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn is_fresh(written_at: u64, now: u64) -> bool {
    now.saturating_sub(written_at) < TTL_SECS
}

pub async fn load_listing() -> Option<Vec<GifItem>> {
    let path = cache_path();
    let bytes = tokio::fs::read(&path).await.ok()?;
    let cache: CacheFile = serde_json::from_slice(&bytes).ok()?;
    if is_fresh(cache.written_at, now_secs()) {
        Some(cache.items)
    } else {
        None
    }
}

pub async fn invalidate_listing() {
    let _ = tokio::fs::remove_file(cache_path()).await;
}

pub async fn save_listing(items: &[GifItem]) -> Result<()> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let cache = CacheFile {
        written_at: now_secs(),
        items: items.to_vec(),
    };
    let bytes = serde_json::to_vec(&cache)?;
    tokio::fs::write(&path, bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fresh_under_ttl() {
        assert!(is_fresh(1000, 1000 + TTL_SECS - 1));
    }

    #[test]
    fn is_fresh_at_boundary_is_stale() {
        assert!(!is_fresh(1000, 1000 + TTL_SECS));
    }

    #[test]
    fn is_fresh_with_same_time() {
        assert!(is_fresh(1000, 1000));
    }

    #[test]
    fn is_fresh_with_future_written_at_saturates_to_zero() {
        // written_at > now means saturating_sub returns 0, which is < TTL
        assert!(is_fresh(2000, 1000));
    }
}
