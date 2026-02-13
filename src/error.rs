use thiserror::Error;

/// Domain-specific errors for the tunnel application.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum CftError {
    #[error("API not configured. Run `tunnel config set` first.")]
    ApiNotConfigured,

    #[error("Zone ID not configured. Run `tunnel config set` first.")]
    ZoneNotConfigured,

    #[error("Cloudflare API error: {message} (code {code})")]
    CloudflareApi { code: u32, message: String },

    #[error("User cancelled the operation")]
    Cancelled,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Convenience alias used throughout the application.
pub type Result<T> = anyhow::Result<T>;
