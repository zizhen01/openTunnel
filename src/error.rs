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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_not_configured_message() {
        assert_eq!(
            CftError::ApiNotConfigured.to_string(),
            "API not configured. Run `tunnel config set` first."
        );
    }

    #[test]
    fn zone_not_configured_message() {
        assert_eq!(
            CftError::ZoneNotConfigured.to_string(),
            "Zone ID not configured. Run `tunnel config set` first."
        );
    }

    #[test]
    fn cloudflare_api_error_message() {
        let e = CftError::CloudflareApi {
            code: 10000,
            message: "Authentication error".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "Cloudflare API error: Authentication error (code 10000)"
        );
    }

    #[test]
    fn cancelled_message() {
        assert_eq!(
            CftError::Cancelled.to_string(),
            "User cancelled the operation"
        );
    }

    #[test]
    fn invalid_input_message() {
        assert_eq!(
            CftError::InvalidInput("bad value".to_string()).to_string(),
            "Invalid input: bad value"
        );
    }
}
