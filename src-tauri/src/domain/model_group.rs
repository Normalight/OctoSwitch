use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGroup {
    pub id: String,
    pub alias: String,
    pub active_binding_id: Option<String>,
    pub is_enabled: bool,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewModelGroup {
    pub alias: String,
}
