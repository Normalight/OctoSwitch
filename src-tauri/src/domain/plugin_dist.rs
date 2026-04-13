use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTaskRouteConfig {
    pub group: String,
    pub member: Option<String>,
    pub delegate_model: Option<String>,
    pub delegate_agent_name: Option<String>,
    pub prompt_template: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub octoswitch_base_url: String,
    pub namespace: String,
    pub default_group: String,
    pub task_routes: std::collections::BTreeMap<String, PluginTaskRouteConfig>,
    pub result_format: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDistBuildResult {
    pub output_path: String,
    pub files: Vec<String>,
    pub plugin_config: Option<PluginConfig>,
}
