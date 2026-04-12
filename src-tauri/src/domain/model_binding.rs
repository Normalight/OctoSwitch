use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBinding {
    pub id: String,
    pub model_name: String,
    pub provider_id: String,
    pub upstream_model_name: String,
    pub input_price_per_1m: f64,
    pub output_price_per_1m: f64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub is_enabled: bool,
    /// 已废弃：仅用旧版 JSON 导入兼容；列表接口始终为 `null`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default)]
    pub group_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewModelBinding {
    pub model_name: String,
    pub provider_id: String,
    pub upstream_model_name: String,
    pub input_price_per_1m: f64,
    pub output_price_per_1m: f64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub is_enabled: bool,
}
