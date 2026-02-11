use anyhow::{bail, Context};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::config::ApiConfig;
use crate::error::{CftError, Result};

const BASE_URL: &str = "https://api.cloudflare.com/client/v4";

// ---------------------------------------------------------------------------
// Generic Cloudflare API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CfResponse<T> {
    pub success: bool,
    pub result: Option<T>,
    #[serde(default)]
    pub errors: Vec<CfApiError>,
    #[serde(default)]
    pub result_info: Option<ResultInfo>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CfApiError {
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResultInfo {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub total_count: Option<u32>,
    pub total_pages: Option<u32>,
}

// ---------------------------------------------------------------------------
// Domain types returned by the API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub created_at: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DnsRecord {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub content: String,
    pub proxied: Option<bool>,
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateDnsRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub proxied: bool,
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessApp {
    pub id: Option<String>,
    pub name: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub app_type: Option<String>,
    pub session_duration: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateAccessApp {
    pub name: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub app_type: String,
    pub session_duration: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessPolicy {
    pub id: Option<String>,
    pub name: String,
    pub decision: String,
    pub include: Vec<PolicyRule>,
    #[serde(default)]
    pub exclude: Vec<PolicyRule>,
    #[serde(default)]
    pub require: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyRule {
    pub email: Option<PolicyEmail>,
    pub email_domain: Option<PolicyEmailDomain>,
    pub everyone: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyEmail {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyEmailDomain {
    pub domain: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// CloudflareClient
// ---------------------------------------------------------------------------

/// Unified HTTP client for all Cloudflare API interactions.
pub struct CloudflareClient {
    http: reqwest::Client,
    pub account_id: String,
    pub zone_id: Option<String>,
}

#[allow(dead_code)]
impl CloudflareClient {
    /// Build a client from a saved `ApiConfig`.
    pub fn from_config(config: &ApiConfig) -> Result<Self> {
        let token = config
            .api_token
            .as_ref()
            .ok_or(CftError::ApiNotConfigured)?;
        let account_id = config
            .account_id
            .as_ref()
            .ok_or(CftError::ApiNotConfigured)?
            .clone();

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .context("invalid token characters")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            account_id,
            zone_id: config.zone_id.clone(),
        })
    }

    // -- helpers ------------------------------------------------------------

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self.http.get(url).send().await.context("HTTP GET failed")?;
        self.parse_response(resp).await
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        let resp = self
            .http
            .post(url)
            .json(body)
            .send()
            .await
            .context("HTTP POST failed")?;
        self.parse_response(resp).await
    }

    async fn put<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        let resp = self
            .http
            .put(url)
            .json(body)
            .send()
            .await
            .context("HTTP PUT failed")?;
        self.parse_response(resp).await
    }

    async fn delete_req<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .http
            .delete(url)
            .send()
            .await
            .context("HTTP DELETE failed")?;
        self.parse_response(resp).await
    }

    async fn parse_response<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let body = resp.text().await.context("failed to read response body")?;

        let cf: CfResponse<T> =
            serde_json::from_str(&body).context("failed to parse Cloudflare response")?;

        if !cf.success {
            let msg = cf
                .errors
                .first()
                .map(|e| format!("{} (code {})", e.message, e.code))
                .unwrap_or_else(|| format!("HTTP {status}"));
            bail!("Cloudflare API error: {msg}");
        }

        cf.result
            .ok_or_else(|| anyhow::anyhow!("empty result from Cloudflare API (HTTP {status})"))
    }

    fn require_zone_id(&self) -> Result<&str> {
        self.zone_id
            .as_deref()
            .ok_or_else(|| CftError::ZoneNotConfigured.into())
    }

    // -- Token verification -------------------------------------------------

    /// Verify the current API token is valid.
    pub async fn verify_token(token: &str) -> Result<bool> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{BASE_URL}/user/tokens/verify"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context("failed to verify token")?;
        Ok(resp.status().is_success())
    }

    /// Fetch all accounts accessible by the token.
    pub async fn fetch_accounts(token: &str) -> Result<Vec<Account>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{BASE_URL}/accounts"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context("failed to fetch accounts")?;
        let body = resp.text().await?;
        let cf: CfResponse<Vec<Account>> = serde_json::from_str(&body)?;
        Ok(cf.result.unwrap_or_default())
    }

    /// Fetch all zones accessible by the token.
    pub async fn fetch_zones(token: &str) -> Result<Vec<Zone>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{BASE_URL}/zones"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context("failed to fetch zones")?;
        let body = resp.text().await?;
        let cf: CfResponse<Vec<Zone>> = serde_json::from_str(&body)?;
        Ok(cf.result.unwrap_or_default())
    }

    // -- Tunnel operations --------------------------------------------------

    /// List all tunnels in the account.
    pub async fn list_tunnels(&self) -> Result<Vec<Tunnel>> {
        let url = format!("{BASE_URL}/accounts/{}/cfd_tunnel", self.account_id);
        self.get(&url).await
    }

    /// Create a new tunnel.
    pub async fn create_tunnel(&self, name: &str, secret: &str) -> Result<Tunnel> {
        let url = format!("{BASE_URL}/accounts/{}/cfd_tunnel", self.account_id);
        let body = serde_json::json!({
            "name": name,
            "tunnel_secret": secret,
        });
        self.post(&url, &body).await
    }

    /// Delete a tunnel by ID.
    pub async fn delete_tunnel(&self, tunnel_id: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{BASE_URL}/accounts/{}/cfd_tunnel/{tunnel_id}",
            self.account_id
        );
        self.delete_req(&url).await
    }

    /// Get tunnel details.
    pub async fn get_tunnel(&self, tunnel_id: &str) -> Result<Tunnel> {
        let url = format!(
            "{BASE_URL}/accounts/{}/cfd_tunnel/{tunnel_id}",
            self.account_id
        );
        self.get(&url).await
    }

    // -- DNS operations -----------------------------------------------------

    /// List DNS records for the configured zone.
    pub async fn list_dns_records(&self) -> Result<Vec<DnsRecord>> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{BASE_URL}/zones/{zone_id}/dns_records?per_page=100");
        self.get(&url).await
    }

    /// Add a DNS record.
    pub async fn create_dns_record(&self, record: &CreateDnsRecord) -> Result<DnsRecord> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{BASE_URL}/zones/{zone_id}/dns_records");
        self.post(&url, record).await
    }

    /// Update a DNS record by ID.
    pub async fn update_dns_record(
        &self,
        record_id: &str,
        record: &CreateDnsRecord,
    ) -> Result<DnsRecord> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{BASE_URL}/zones/{zone_id}/dns_records/{record_id}");
        self.put(&url, record).await
    }

    /// Delete a DNS record by ID.
    pub async fn delete_dns_record(&self, record_id: &str) -> Result<serde_json::Value> {
        let zone_id = self.require_zone_id()?;
        let url = format!("{BASE_URL}/zones/{zone_id}/dns_records/{record_id}");
        self.delete_req(&url).await
    }

    // -- Access operations --------------------------------------------------

    /// List Access applications.
    pub async fn list_access_apps(&self) -> Result<Vec<AccessApp>> {
        let url = format!("{BASE_URL}/accounts/{}/access/apps", self.account_id);
        self.get(&url).await
    }

    /// Create an Access application.
    pub async fn create_access_app(&self, app: &CreateAccessApp) -> Result<AccessApp> {
        let url = format!("{BASE_URL}/accounts/{}/access/apps", self.account_id);
        self.post(&url, app).await
    }

    /// Delete an Access application.
    pub async fn delete_access_app(&self, app_id: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{BASE_URL}/accounts/{}/access/apps/{app_id}",
            self.account_id
        );
        self.delete_req(&url).await
    }

    /// List policies for an Access application.
    pub async fn list_access_policies(&self, app_id: &str) -> Result<Vec<AccessPolicy>> {
        let url = format!(
            "{BASE_URL}/accounts/{}/access/apps/{app_id}/policies",
            self.account_id
        );
        self.get(&url).await
    }

    /// Create a policy for an Access application.
    pub async fn create_access_policy(
        &self,
        app_id: &str,
        policy: &AccessPolicy,
    ) -> Result<AccessPolicy> {
        let url = format!(
            "{BASE_URL}/accounts/{}/access/apps/{app_id}/policies",
            self.account_id
        );
        self.post(&url, policy).await
    }
}
