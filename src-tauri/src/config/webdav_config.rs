use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;

const WEBDAV_CONFIG_FILE: &str = "webdav.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub remote_root: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            username: String::new(),
            password: String::new(),
            remote_root: "octoswitch-sync".to_string(),
        }
    }
}

impl WebDavConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        let url = self.base_url.trim();
        if url.is_empty() {
            return Err(AppError::Internal("WebDAV 地址不能为空".into()));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AppError::Internal("WebDAV 地址必须以 http:// 或 https:// 开头".into()));
        }
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.username.trim().is_empty()
    }
}

fn config_file_path() -> PathBuf {
    super::app_config::config_dir().join(WEBDAV_CONFIG_FILE)
}

pub fn load() -> Option<WebDavConfig> {
    let path = config_file_path();
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn save(config: &WebDavConfig) -> Result<(), AppError> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("创建 WebDAV 配置目录失败: {e}")))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Internal(format!("序列化 WebDAV 配置失败: {e}")))?;
    fs::write(&path, json)
        .map_err(|e| AppError::Internal(format!("写入 WebDAV 配置失败: {e}")))?;
    Ok(())
}
