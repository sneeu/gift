use anyhow::{Context, Result};
use aws_config::Region;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;

use crate::app::GifItem;
use crate::config::Config;

pub async fn build_client(config: &Config) -> Client {
    // `aws_config::defaults` loads the full credential chain: explicit provider (if set),
    // then env vars (AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY), then ~/.aws/credentials,
    // then IAM instance roles, etc.  This matches what the AWS CLI does.
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(Region::new(config.aws_region.clone()));

    if config.has_explicit_credentials() {
        let credentials = Credentials::new(
            &config.aws_access_key,
            &config.aws_secret_key,
            None,
            None,
            "gift-config",
        );
        loader = loader.credentials_provider(SharedCredentialsProvider::new(credentials));
    }

    let sdk_config = loader.load().await;

    // Force path-style addressing so buckets with dots in the name (e.g. "my.bucket.com")
    // don't break TLS — virtual-hosted-style produces "my.bucket.com.s3.amazonaws.com"
    // which isn't covered by the wildcard cert.
    let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .force_path_style(true)
        .build();
    Client::from_conf(s3_config)
}

/// List all GIF objects in the bucket (handles pagination).
pub async fn list_all(client: &Client, bucket: &str) -> Result<Vec<GifItem>> {
    let mut items = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut req = client.list_objects_v2().bucket(bucket);
        if let Some(ref token) = continuation_token {
            req = req.continuation_token(token);
        }

        let resp = req
            .send()
            .await
            .with_context(|| format!("Failed to list objects in bucket '{bucket}'"))?;

        for obj in resp.contents() {
            let key = obj.key().unwrap_or("").to_owned();
            if key.is_empty() {
                continue;
            }
            let size = obj.size().unwrap_or(0) as u64;
            let last_modified = obj
                .last_modified()
                .map(|dt| dt.to_string())
                .unwrap_or_default();
            items.push(GifItem {
                key,
                size,
                last_modified,
            });
        }

        match resp.next_continuation_token() {
            Some(token) => continuation_token = Some(token.to_owned()),
            None => break,
        }
    }

    Ok(items)
}

/// Upload bytes to S3 as a GIF object.
pub async fn upload(client: &Client, bucket: &str, key: &str, data: Bytes) -> Result<()> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type("image/gif")
        .body(ByteStream::from(data))
        .send()
        .await
        .with_context(|| format!("Failed to upload '{key}' to bucket '{bucket}'"))?;
    Ok(())
}

/// Rename an object by copying then deleting the original.
/// If the copy fails, the original is left intact.
pub async fn rename(client: &Client, bucket: &str, old_key: &str, new_key: &str) -> Result<()> {
    let copy_source = format!("{bucket}/{old_key}");
    client
        .copy_object()
        .bucket(bucket)
        .copy_source(&copy_source)
        .key(new_key)
        .send()
        .await
        .with_context(|| format!("Failed to copy '{old_key}' to '{new_key}'"))?;

    // Only delete if copy succeeded
    client
        .delete_object()
        .bucket(bucket)
        .key(old_key)
        .send()
        .await
        .with_context(|| format!("Failed to delete original '{old_key}' after rename"))?;

    Ok(())
}

/// Delete an object from S3.
pub async fn delete(client: &Client, bucket: &str, key: &str) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .with_context(|| format!("Failed to delete '{key}' from bucket '{bucket}'"))?;
    Ok(())
}
