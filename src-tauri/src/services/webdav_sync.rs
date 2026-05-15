use crate::config::webdav_config::WebDavConfig;
use crate::domain::error::AppError;
use crate::services::webdav::{
    auth_from_credentials, build_remote_url, ensure_remote_directories, get_bytes, head_etag,
    path_segments, put_bytes, test_connection,
};

use serde_json::Value;

const REMOTE_FILE: &str = "octoswitch-config.json";

fn remote_dir_segments(config: &WebDavConfig) -> Vec<&str> {
    let mut segs: Vec<&str> = path_segments(&config.remote_root).collect();
    if segs.is_empty() {
        segs.push("octoswitch-sync");
    }
    segs
}

fn auth_for(config: &WebDavConfig) -> super::webdav::WebDavAuth {
    auth_from_credentials(&config.username, &config.password)
}

pub async fn check_connection(
    client: &reqwest::Client,
    config: &WebDavConfig,
) -> Result<(), AppError> {
    config.validate()?;
    let auth = auth_for(config);
    test_connection(client, &config.base_url, &auth).await?;
    let segs = remote_dir_segments(config);
    ensure_remote_directories(client, &config.base_url, &segs, &auth).await?;
    Ok(())
}

pub async fn upload(
    client: &reqwest::Client,
    config: &WebDavConfig,
    config_json: String,
) -> Result<Value, AppError> {
    config.validate()?;
    let auth = auth_for(config);
    let segs = remote_dir_segments(config);
    ensure_remote_directories(client, &config.base_url, &segs, &auth).await?;

    let file_url = {
        let mut parts = segs.to_vec();
        parts.push(REMOTE_FILE);
        build_remote_url(&config.base_url, &parts)?
    };

    put_bytes(
        client,
        &file_url,
        &auth,
        config_json.into_bytes(),
        "application/json",
    )
    .await?;

    log::info!("[WebDAV] Upload ok: {}", file_url);
    Ok(serde_json::json!({ "status": "uploaded" }))
}

pub async fn download(
    client: &reqwest::Client,
    config: &WebDavConfig,
) -> Result<String, AppError> {
    config.validate()?;
    let auth = auth_for(config);

    let file_url = {
        let mut parts: Vec<&str> = path_segments(&config.remote_root).collect();
        if parts.is_empty() {
            parts.push("octoswitch-sync");
        }
        parts.push(REMOTE_FILE);
        build_remote_url(&config.base_url, &parts)?
    };

    let bytes = get_bytes(client, &file_url, &auth)
        .await?
        .ok_or_else(|| {
            AppError::Internal("远端没有可下载的配置文件".into())
        })?;

    let text = String::from_utf8(bytes)
        .map_err(|e| AppError::Internal(format!("远端配置文件不是有效的 UTF-8: {e}")))?;

    log::info!("[WebDAV] Download ok: {} bytes", text.len());
    Ok(text)
}

pub async fn fetch_remote_info(
    client: &reqwest::Client,
    config: &WebDavConfig,
) -> Result<Option<Value>, AppError> {
    config.validate()?;
    let auth = auth_for(config);

    let file_url = {
        let mut parts: Vec<&str> = path_segments(&config.remote_root).collect();
        if parts.is_empty() {
            parts.push("octoswitch-sync");
        }
        parts.push(REMOTE_FILE);
        build_remote_url(&config.base_url, &parts)?
    };

    let info = head_etag(client, &file_url, &auth).await?;
    match info {
        None => Ok(None),
        Some((_, last_modified)) => Ok(Some(serde_json::json!({
            "exists": true,
            "lastModified": last_modified,
            "remotePath": format!("/{}/{}", remote_dir_segments(config).join("/"), REMOTE_FILE),
        }))),
    }
}
