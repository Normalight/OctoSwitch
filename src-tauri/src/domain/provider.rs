use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key_ref: String,
    pub timeout_ms: i64,
    pub max_retries: i64,
    pub is_enabled: bool,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub api_format: Option<String>,
    #[serde(default = "default_auth_mode")]
    pub auth_mode: String,
}

impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("base_url", &self.base_url)
            .field("api_key_ref", &mask_key(&self.api_key_ref))
            .field("timeout_ms", &self.timeout_ms)
            .field("max_retries", &self.max_retries)
            .field("is_enabled", &self.is_enabled)
            .field("sort_order", &self.sort_order)
            .field("api_format", &self.api_format)
            .field("auth_mode", &self.auth_mode)
            .finish()
    }
}

impl Provider {
    pub fn to_summary(&self) -> ProviderSummary {
        ProviderSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            api_key_masked: mask_key(&self.api_key_ref),
            timeout_ms: self.timeout_ms,
            max_retries: self.max_retries,
            is_enabled: self.is_enabled,
            sort_order: self.sort_order,
            api_format: self.api_format.clone(),
            auth_mode: self.auth_mode.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key_masked: String,
    pub timeout_ms: i64,
    pub max_retries: i64,
    pub is_enabled: bool,
    pub sort_order: i64,
    pub api_format: Option<String>,
    pub auth_mode: String,
}

/// Mask an API key, showing only the last 4 characters.
/// Short keys (<= 8 chars) are fully masked.
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}...{}", &key[..2], &key[key.len() - 4..])
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NewProvider {
    pub name: String,
    pub base_url: String,
    pub api_key_ref: String,
    pub timeout_ms: i64,
    pub max_retries: i64,
    pub is_enabled: bool,
    #[serde(default)]
    pub api_format: Option<String>,
    #[serde(default = "default_auth_mode")]
    pub auth_mode: String,
}

impl std::fmt::Debug for NewProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewProvider")
            .field("name", &self.name)
            .field("base_url", &self.base_url)
            .field("api_key_ref", &mask_key(&self.api_key_ref))
            .field("timeout_ms", &self.timeout_ms)
            .field("max_retries", &self.max_retries)
            .field("is_enabled", &self.is_enabled)
            .field("api_format", &self.api_format)
            .field("auth_mode", &self.auth_mode)
            .finish()
    }
}

fn default_auth_mode() -> String {
    "bearer".to_string()
}
