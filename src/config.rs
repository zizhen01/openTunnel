use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use crate::error::Result;

// ---------------------------------------------------------------------------
// API config (~/.cft/config.json)
// ---------------------------------------------------------------------------

/// Stored credentials and user preferences.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl ApiConfig {
    /// Mask token for display, e.g. `cf_***...***`.
    pub fn masked_token(&self) -> String {
        match &self.api_token {
            Some(t) if t.chars().count() > 8 => {
                let prefix: String = t.chars().take(4).collect();
                let suffix: String = t
                    .chars()
                    .rev()
                    .take(4)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("{prefix}***...***{suffix}")
            }
            Some(_) => "****".to_string(),
            None => "not set".to_string(),
        }
    }
}

/// Return the path to the cft config directory (`~/.cft`).
pub fn cft_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(home.join(".cft"))
}

/// Return the path to `~/.cft/config.json`.
pub fn api_config_path() -> Result<PathBuf> {
    Ok(cft_config_dir()?.join("config.json"))
}

/// Load the API config from disk. Returns `None` if the file does not exist.
pub fn load_api_config() -> Result<Option<ApiConfig>> {
    let path = api_config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: ApiConfig = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(cfg))
}

/// Save the API config to disk with secure file permissions (0600).
pub fn save_api_config(config: &ApiConfig) -> Result<()> {
    let dir = cft_config_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let path = api_config_path()?;
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    set_api_config_permissions(&path)?;

    Ok(())
}

#[cfg(unix)]
fn set_api_config_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Secure the file: owner read/write only
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_api_config_permissions(_path: &std::path::Path) -> Result<()> {
    // Windows and other non-Unix platforms do not support Unix mode bits.
    Ok(())
}

/// Delete the API config file.
pub fn clear_api_config() -> Result<()> {
    let path = api_config_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Quick check: is the API configured (token + account_id present)?
pub fn is_api_configured() -> bool {
    load_api_config()
        .ok()
        .flatten()
        .map(|c| c.api_token.is_some() && c.account_id.is_some())
        .unwrap_or(false)
}

/// Load and return ApiConfig, or bail with a helpful message.
pub fn require_api_config() -> Result<ApiConfig> {
    match load_api_config()? {
        Some(ref c) if c.api_token.is_some() && c.account_id.is_some() => Ok(c.clone()),
        _ => bail!(crate::error::CftError::ApiNotConfigured),
    }
}

/// Load and return ApiConfig with zone_id present, or bail.
pub fn require_zone_config() -> Result<ApiConfig> {
    let cfg = require_api_config()?;
    if cfg.zone_id.is_none() {
        bail!(crate::error::CftError::ZoneNotConfigured);
    }
    Ok(cfg)
}

// ---------------------------------------------------------------------------
// Tunnel config (/etc/cloudflared/config.yml or platform equivalent)
// ---------------------------------------------------------------------------

/// Cloudflared tunnel config (the YAML file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub tunnel: String,
    #[serde(rename = "credentials-file")]
    pub credentials_file: String,
    #[serde(default)]
    pub ingress: Vec<IngressRule>,
}

/// A single ingress rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub service: String,
}

/// Return the platform-appropriate cloudflared config path.
pub fn tunnel_config_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir()
            .map(|h| h.join(".cloudflared").join("config.yml"))
            .unwrap_or_else(|| PathBuf::from("/etc/cloudflared/config.yml"))
    } else {
        PathBuf::from("/etc/cloudflared/config.yml")
    }
}

/// Load the tunnel config YAML.
pub fn load_tunnel_config() -> Result<TunnelConfig> {
    let path = tunnel_config_path();
    if !path.exists() {
        bail!(crate::error::CftError::ConfigNotFound {
            path: path.display().to_string()
        });
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: TunnelConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(cfg)
}

/// Save the tunnel config YAML back to disk.
pub fn save_tunnel_config(config: &TunnelConfig) -> Result<()> {
    let path = tunnel_config_path();
    let yaml = serde_yaml::to_string(config)?;
    fs::write(&path, &yaml).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Return the list of currently configured hostnames (excluding catch-all).
pub fn configured_hostnames(config: &TunnelConfig) -> Vec<String> {
    config
        .ingress
        .iter()
        .filter_map(|r| r.hostname.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_token_display() {
        let cfg = ApiConfig {
            api_token: Some("abcdefghijklmnop".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.masked_token(), "abcd***...***mnop");
    }

    #[test]
    fn masked_token_short() {
        let cfg = ApiConfig {
            api_token: Some("short".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.masked_token(), "****");
    }

    #[test]
    fn masked_token_unicode_safe() {
        let cfg = ApiConfig {
            api_token: Some("测a试b字c符d串e".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.masked_token(), "测a试b***...***符d串e");
    }

    #[test]
    fn masked_token_none() {
        let cfg = ApiConfig::default();
        assert_eq!(cfg.masked_token(), "not set");
    }

    #[test]
    fn parse_tunnel_config_yaml() {
        let yaml = r#"
tunnel: abc-123
credentials-file: /root/.cloudflared/abc-123.json
ingress:
  - hostname: app.example.com
    service: http://localhost:3000
  - service: http_status:404
"#;
        let cfg: TunnelConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.tunnel, "abc-123");
        assert_eq!(cfg.ingress.len(), 2);
        assert_eq!(cfg.ingress[0].hostname.as_deref(), Some("app.example.com"));
        assert!(cfg.ingress[1].hostname.is_none());
    }
}
