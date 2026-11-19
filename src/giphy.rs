use anyhow::{Context, Result};
use serde::Deserialize;

const SEARCH_URL: &str = "https://api.giphy.com/v1/gifs/search";
const LIMIT: &str = "50";

#[derive(Debug, Clone, PartialEq)]
pub struct GiphyGif {
    pub id: String,
    pub title: String,
    /// Small rendition for previewing.
    pub preview_url: String,
    /// Full-size rendition for uploading.
    pub original_url: String,
}

impl GiphyGif {
    /// Key used for the preview frame caches; namespaced so it can't clash with S3 keys.
    pub fn cache_key(&self) -> String {
        format!("giphy/{}", self.id)
    }

    /// Suggested S3 name, derived from the title (falling back to the ID).
    pub fn suggested_name(&self) -> String {
        let slug = slugify(strip_gif_suffix(&self.title));
        if slug.is_empty() { self.id.clone() } else { slug }
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<RawGif>,
}

#[derive(Deserialize)]
struct RawGif {
    id: String,
    #[serde(default)]
    title: String,
    images: RawImages,
}

#[derive(Deserialize)]
struct RawImages {
    original: RawImage,
    fixed_height: RawImage,
}

#[derive(Deserialize)]
struct RawImage {
    url: String,
}

pub async fn search(api_key: &str, query: &str) -> Result<Vec<GiphyGif>> {
    let url = reqwest::Url::parse_with_params(
        SEARCH_URL,
        &[("api_key", api_key), ("q", query), ("limit", LIMIT)],
    )?;
    let response = reqwest::get(url).await.context("Giphy request failed")?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Giphy returned HTTP {status}");
    }
    let bytes = response.bytes().await.context("Failed to read Giphy response")?;
    parse_search(&bytes)
}

fn parse_search(bytes: &[u8]) -> Result<Vec<GiphyGif>> {
    let response: SearchResponse =
        serde_json::from_slice(bytes).context("Malformed Giphy response")?;
    Ok(response
        .data
        .into_iter()
        .map(|g| GiphyGif {
            id: g.id,
            title: g.title,
            preview_url: g.images.fixed_height.url,
            original_url: g.images.original.url,
        })
        .collect())
}

/// Giphy titles look like "Happy Cat GIF by Someone"; drop everything from " GIF".
fn strip_gif_suffix(title: &str) -> &str {
    title.find(" GIF").map_or(title, |i| &title[..i])
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gif(id: &str, title: &str) -> GiphyGif {
        GiphyGif {
            id: id.into(),
            title: title.into(),
            preview_url: String::new(),
            original_url: String::new(),
        }
    }

    #[test]
    fn suggested_name_strips_suffix_and_slugifies() {
        assert_eq!(gif("x", "Happy Cat GIF by Someone").suggested_name(), "happy-cat");
        assert_eq!(gif("x", "Mind Blown GIF").suggested_name(), "mind-blown");
        assert_eq!(gif("x", "It's  a trap!").suggested_name(), "it-s-a-trap");
    }

    #[test]
    fn suggested_name_falls_back_to_id() {
        assert_eq!(gif("abc123", "").suggested_name(), "abc123");
        assert_eq!(gif("abc123", " GIF by Someone").suggested_name(), "abc123");
    }

    #[test]
    fn cache_key_is_namespaced() {
        assert_eq!(gif("abc", "").cache_key(), "giphy/abc");
    }

    #[test]
    fn parse_search_extracts_urls() {
        let json = br#"{
            "data": [{
                "id": "abc",
                "title": "Cat GIF",
                "images": {
                    "original": { "url": "https://media.giphy.com/abc/giphy.gif" },
                    "fixed_height": { "url": "https://media.giphy.com/abc/200.gif" }
                }
            }],
            "meta": { "status": 200 }
        }"#;
        let gifs = parse_search(json).unwrap();
        assert_eq!(gifs.len(), 1);
        assert_eq!(gifs[0].id, "abc");
        assert_eq!(gifs[0].original_url, "https://media.giphy.com/abc/giphy.gif");
        assert_eq!(gifs[0].preview_url, "https://media.giphy.com/abc/200.gif");
    }

    #[test]
    fn parse_search_rejects_garbage() {
        assert!(parse_search(b"nope").is_err());
    }
}
