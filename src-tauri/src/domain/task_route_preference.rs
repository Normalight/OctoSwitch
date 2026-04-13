use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRoutePreference {
    pub id: String,
    pub task_kind: String,
    pub target_group: String,
    pub target_member: Option<String>,
    pub prompt_template: Option<String>,
    pub is_enabled: bool,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskRoutePreference {
    pub task_kind: String,
    pub target_group: String,
    pub target_member: Option<String>,
    pub prompt_template: Option<String>,
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,
}

fn default_enabled() -> bool {
    true
}
