use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPluginStatus {
    pub marketplace_path: String,
    pub marketplace_repo: String,
    pub tracked_path: String,
    pub installed_path: String,
    pub tracked_exists: bool,
    pub installed_exists: bool,
    pub up_to_date: bool,
    pub tracked_file_count: usize,
    pub installed_file_count: usize,
    pub missing_files: Vec<String>,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPluginSyncResult {
    pub status: LocalPluginStatus,
    pub copied_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub preserved_files: Vec<String>,
}
