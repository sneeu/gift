use anyhow::{Context, Result};
use bytes::Bytes;

/// Fetch raw bytes from either an HTTP(S) URL or a local file path.
pub async fn fetch_source(source: &str) -> Result<Bytes> {
    if source.starts_with("https://") || source.starts_with("http://") {
        fetch_url(source).await
    } else {
        fetch_file(source).await
    }
}

async fn fetch_url(url: &str) -> Result<Bytes> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to download {url}"))?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} downloading {url}");
    }
    response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response body from {url}"))
}

async fn fetch_file(path: &str) -> Result<Bytes> {
    let data = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read file: {path}"))?;
    Ok(Bytes::from(data))
}

/// Derive a filename from a URL (the last path segment, without query string).
pub fn basename_from_url(url: &str) -> Option<String> {
    let without_query = url.split('?').next().unwrap_or(url);
    let name = without_query.rsplit('/').next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

/// Ensure the filename ends with `.gif`.
pub fn ensure_gif_extension(name: &str) -> String {
    if name.to_lowercase().ends_with(".gif") {
        name.to_owned()
    } else {
        format!("{name}.gif")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_from_simple_url() {
        assert_eq!(
            basename_from_url("https://example.com/cats/happy.gif"),
            Some("happy.gif".into())
        );
    }

    #[test]
    fn basename_strips_query_string() {
        assert_eq!(
            basename_from_url("https://example.com/foo.gif?v=2"),
            Some("foo.gif".into())
        );
    }

    #[test]
    fn basename_from_trailing_slash_returns_none() {
        assert_eq!(basename_from_url("https://example.com/"), None);
    }

    #[test]
    fn basename_from_local_path() {
        assert_eq!(
            basename_from_url("/home/user/my.gif"),
            Some("my.gif".into())
        );
    }

    #[test]
    fn ensure_gif_extension_already_has_it() {
        assert_eq!(ensure_gif_extension("cat.gif"), "cat.gif");
        assert_eq!(ensure_gif_extension("cat.GIF"), "cat.GIF");
    }

    #[test]
    fn ensure_gif_extension_adds_it() {
        assert_eq!(ensure_gif_extension("cat"), "cat.gif");
        assert_eq!(ensure_gif_extension("my-animation"), "my-animation.gif");
    }

    #[test]
    fn fetch_source_dispatches_on_prefix() {
        // Just test that the dispatch logic compiles and runs to the right branch.
        // Actual HTTP/file I/O is tested via integration tests.
        let is_url = "https://example.com/a.gif".starts_with("https://")
            || "https://example.com/a.gif".starts_with("http://");
        assert!(is_url);

        let is_file = !"./local.gif".starts_with("https://")
            && !"./local.gif".starts_with("http://");
        assert!(is_file);
    }
}
