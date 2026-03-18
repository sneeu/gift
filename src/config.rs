use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bucket: String,
    pub base_url: String,
    pub aws_region: String,
    /// If omitted, the AWS SDK credential chain is used instead
    /// (env vars AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY, ~/.aws/credentials, etc.).
    #[serde(default)]
    pub aws_access_key: String,
    #[serde(default)]
    pub aws_secret_key: String,
}

impl Config {
    /// True when explicit credentials were supplied in the config file.
    pub fn has_explicit_credentials(&self) -> bool {
        !self.aws_access_key.is_empty() && !self.aws_secret_key.is_empty()
    }
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    let contents = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Cannot read config file at {}\n\n\
             Create ~/.config/gift.toml with:\n\n\
             \tbucket = \"my-bucket\"\n\
             \tbase_url = \"https://example.com\"\n\
             \taws_region = \"us-east-1\"\n\
             \t# Credentials are optional here — the AWS SDK will also check\n\
             \t# AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY env vars and ~/.aws/credentials\n\
             \t# aws_access_key = \"AKID...\"\n\
             \t# aws_secret_key = \"...\"",
            path.display()
        )
    })?;

    let config: Config = toml::from_str(&contents)
        .with_context(|| format!("Malformed config at {}", path.display()))?;

    validate(&config)?;
    Ok(config)
}

fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".config").join("gift.toml"))
}

fn validate(config: &Config) -> Result<()> {
    if config.bucket.is_empty() {
        bail!("Config: 'bucket' is required");
    }
    if config.base_url.is_empty() {
        bail!("Config: 'base_url' is required");
    }
    if config.aws_region.is_empty() {
        bail!("Config: 'aws_region' is required");
    }
    // Only one of the two credential fields is set — that's likely a mistake.
    let has_key = !config.aws_access_key.is_empty();
    let has_secret = !config.aws_secret_key.is_empty();
    if has_key != has_secret {
        bail!(
            "Config: set both aws_access_key and aws_secret_key, or neither \
             (to use AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY env vars)."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(access: &str, secret: &str) -> Config {
        Config {
            bucket: "b".into(),
            base_url: "https://x.com".into(),
            aws_region: "us-east-1".into(),
            aws_access_key: access.into(),
            aws_secret_key: secret.into(),
        }
    }

    #[test]
    fn validate_ok_with_both_credentials() {
        assert!(validate(&make_config("id", "secret")).is_ok());
    }

    #[test]
    fn validate_ok_with_no_credentials() {
        // Credentials will come from env / SDK chain at runtime
        assert!(validate(&make_config("", "")).is_ok());
    }

    #[test]
    fn validate_err_with_only_one_credential() {
        assert!(validate(&make_config("id", "")).is_err());
        assert!(validate(&make_config("", "secret")).is_err());
    }

    #[test]
    fn validate_missing_bucket() {
        let mut c = make_config("id", "secret");
        c.bucket = String::new();
        assert!(validate(&c).is_err());
    }

    #[test]
    fn validate_missing_base_url() {
        let mut c = make_config("id", "secret");
        c.base_url = String::new();
        assert!(validate(&c).is_err());
    }

    #[test]
    fn parse_toml_round_trip() {
        let raw = r#"
            bucket = "my-bucket"
            base_url = "https://cdn.example.com"
            aws_region = "eu-west-1"
            aws_access_key = "AKID1234"
            aws_secret_key = "secret1234"
        "#;
        let config: Config = toml::from_str(raw).unwrap();
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.aws_region, "eu-west-1");
        assert!(config.has_explicit_credentials());
    }

    #[test]
    fn parse_toml_without_credentials() {
        let raw = r#"
            bucket = "b"
            base_url = "https://x.com"
            aws_region = "us-east-1"
        "#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(!config.has_explicit_credentials());
    }
}
