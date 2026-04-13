use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDistBuildResult {
    pub output_path: String,
    pub files: Vec<String>,
}
