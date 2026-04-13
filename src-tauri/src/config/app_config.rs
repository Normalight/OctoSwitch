use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub gateway_port: u16,
    pub gateway_host: String,
    pub db_path: String,
    pub http_proxy: Option<String>,
}

fn env_octoswitch_or_legacy_db_path() -> Option<String> {
    std::env::var("OCTOSWITCH_DB_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("MG_DB_PATH")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
}

fn env_octoswitch_or_legacy_http_proxy() -> Option<String> {
    std::env::var("OCTOSWITCH_HTTP_PROXY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("MG_HTTP_PROXY")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_port: 8787,
            gateway_host: "127.0.0.1".to_string(),
            db_path: env_octoswitch_or_legacy_db_path().unwrap_or_else(|| {
                config_dir()
                    .join("octoswitch.db")
                    .to_string_lossy()
                    .into_owned()
            }),
            http_proxy: env_octoswitch_or_legacy_http_proxy(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    /// 关闭窗口时隐藏到系统托盘（最小化仍留在任务栏；与前端 `close_to_tray` 一致；`minimize_to_tray` 为旧版 JSON 字段名）
    #[serde(default, alias = "minimize_to_tray")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub auto_start: bool,
    /// 开机自启动且进程由自启动项拉起时，启动后不显示主窗口（仅托盘）；依赖自启动参数 `--octoswitch-autostart`
    #[serde(default)]
    pub silent_autostart: bool,
    /// 可选：关闭到托盘时销毁主窗口（省内存，托盘恢复需重建）；关闭则用 hide，恢复更快（类似 cc-switch 默认）
    #[serde(default)]
    pub light_tray_mode: bool,
    /// 允许客户端使用 `分组别名/绑定路由名`；关闭时仅能用分组别名，且 `GET /v1/models` 只列出分组
    #[serde(
        default = "default_allow_group_member_model_path",
        alias = "allowGroupMemberModelPath"
    )]
    pub allow_group_member_model_path: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub skills_enabled: bool,
    #[serde(default = "default_plugin_enabled")]
    pub plugin_enabled: bool,
    #[serde(default = "default_plugin_namespace")]
    pub plugin_namespace: String,
    #[serde(default = "default_plugin_dist_path")]
    pub plugin_dist_path: String,
    #[serde(default)]
    pub marketplace_enabled: bool,
    #[serde(default = "default_skills_source_path")]
    pub skills_source_path: String,
    #[serde(default = "default_claude_skills_path")]
    pub claude_skills_path: String,
    /// 用户忽略的更新版本号（下次检查时跳过此版本）
    #[serde(default)]
    pub ignored_update_version: Option<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_allow_group_member_model_path() -> bool {
    true
}

fn default_plugin_enabled() -> bool {
    true
}

fn default_plugin_namespace() -> String {
    "octoswitch".to_string()
}

fn default_plugin_dist_path() -> String {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("plugin-dist")
        .to_string_lossy()
        .into_owned()
}

fn default_skills_source_path() -> String {
    cc_switch_skills_dir().to_string_lossy().into_owned()
}

fn default_claude_skills_path() -> String {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claude")
        .join("skills")
        .to_string_lossy()
        .into_owned()
}

pub fn cc_switch_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch")
        .join("skills")
}

pub fn cc_switch_plugins_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch")
        .join("plugins")
}

pub fn repo_root_skills_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap_or(manifest_dir.as_path());
    repo_root.join("skills")
}

pub fn repo_root_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap_or(manifest_dir.as_path())
        .to_path_buf()
}

pub fn repo_root_marketplace_manifest_path() -> PathBuf {
    repo_root_dir().join(".claude-plugin").join("marketplace.json")
}

impl GatewayConfig {
    pub fn log_level_filter(&self) -> log::LevelFilter {
        match self.log_level.to_lowercase().as_str() {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            "off" => log::LevelFilter::Off,
            _ => log::LevelFilter::Info,
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8787,
            close_to_tray: false,
            auto_start: false,
            silent_autostart: false,
            light_tray_mode: false,
            allow_group_member_model_path: default_allow_group_member_model_path(),
            log_level: default_log_level(),
            debug_mode: false,
            skills_enabled: false,
            plugin_enabled: default_plugin_enabled(),
            plugin_namespace: default_plugin_namespace(),
            plugin_dist_path: default_plugin_dist_path(),
            marketplace_enabled: false,
            skills_source_path: default_skills_source_path(),
            claude_skills_path: default_claude_skills_path(),
            ignored_update_version: None,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("OctoSwitch")
}

/// 旧版（LiteLLM 等）与本机数据目录名；首次启动时用于迁移 `gateway_config.json` 与数据库。
pub fn legacy_app_data_dir_litellm() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("LiteLLM")
}

fn config_file_path() -> PathBuf {
    config_dir().join("gateway_config.json")
}

pub fn load_gateway_config() -> GatewayConfig {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(_) => GatewayConfig::default(),
        },
        Err(_) => {
            let legacy = legacy_app_data_dir_litellm().join("gateway_config.json");
            match fs::read_to_string(&legacy) {
                Ok(contents) => match serde_json::from_str::<GatewayConfig>(&contents) {
                    Ok(config) => {
                        let _ = save_gateway_config(&config);
                        config
                    }
                    Err(_) => GatewayConfig::default(),
                },
                Err(_) => GatewayConfig::default(),
            }
        }
    }
}

pub fn save_gateway_config(config: &GatewayConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(config_file_path(), json).map_err(|e| format!("Failed to write config: {e}"))
}
