# gift: gif thing

It’s a TUI for managing gifs stored on AWS S3 and served by AWS CloudFront. It has upload, rename, search, preview, and URL copying. It’s pretty much all I need to manage the gifs in my S3 bucket.

I think it looks pretty cool:

![Screenshot of gift running in Ghostty on macOS](docs/screenshot.png)

## Install

Install with:

```sh
brew install sneeu/tap/gift
```

## Giphy

Press `g` to search Giphy, then `Enter` on a result to upload it. Requires an API key from the [Giphy developer dashboard](https://developers.giphy.com/dashboard/) in `~/.config/gift.toml`:

```toml
giphy_api_key = "..."
```
