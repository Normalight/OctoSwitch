use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_auth_mode() -> String {
    "bearer".to_string()
}
