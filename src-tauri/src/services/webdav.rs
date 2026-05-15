use std::time::Duration;

use reqwest::{Method, RequestBuilder, StatusCode, Url};

use crate::domain::error::AppError;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const TRANSFER_TIMEOUT_SECS: u64 = 120;

pub type WebDavAuth = Option<(String, Option<String>)>;

fn method_propfind() -> Method {
    Method::from_bytes(b"PROPFIND").expect("PROPFIND is valid")
}

fn method_mkcol() -> Method {
    Method::from_bytes(b"MKCOL").expect("MKCOL is valid")
}

pub fn parse_base_url(raw: &str) -> Result<Url, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("WebDAV 地址不能为空".into()));
    }
    Url::parse(trimmed).map_err(|e| AppError::Internal(format!("WebDAV 地址无效: {e}")))?;
    let url = Url::parse(trimmed).map_err(|e| AppError::Internal(format!("WebDAV 地址无效: {e}")))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(AppError::Internal("WebDAV 仅支持 http/https 地址".into())),
    }
}

pub fn build_remote_url(base_url: &str, segments: &[&str]) -> Result<String, AppError> {
    let mut url = parse_base_url(base_url)?;
    {
        let mut path = url.path_segments_mut().map_err(|_| {
            AppError::Internal("WebDAV 地址格式不支持追加路径".into())
        })?;
        path.pop_if_empty();
        for seg in segments {
            path.push(seg);
        }
    }
    Ok(url.to_string())
}

pub fn path_segments(raw: &str) -> impl Iterator<Item = &str> {
    raw.trim_matches('/').split('/').filter(|s| !s.is_empty())
}

pub fn auth_from_credentials(username: &str, password: &str) -> WebDavAuth {
    let user = username.trim();
    if user.is_empty() {
        return None;
    }
    Some((user.to_string(), Some(password.to_string())))
}

fn apply_auth(builder: RequestBuilder, auth: &WebDavAuth) -> RequestBuilder {
    match auth {
        Some((user, pass)) => builder.basic_auth(user, pass.as_deref()),
        None => builder,
    }
}

pub async fn test_connection(
    client: &reqwest::Client,
    base_url: &str,
    auth: &WebDavAuth,
) -> Result<(), AppError> {
    let url = parse_base_url(base_url)?;
    let resp = apply_auth(
        client
            .request(method_propfind(), url)
            .header("Depth", "0")
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
        auth,
    )
    .send()
    .await
    .map_err(|e| AppError::Http(format!("WebDAV 连接失败: {e}")))?;

    if resp.status().is_success() || resp.status() == StatusCode::MULTI_STATUS {
        return Ok(());
    }
    Err(webdav_status_error("PROPFIND", resp.status(), base_url))
}

pub async fn ensure_remote_directories(
    client: &reqwest::Client,
    base_url: &str,
    segments: &[&str],
    auth: &WebDavAuth,
) -> Result<(), AppError> {
    if segments.is_empty() {
        return Ok(());
    }

    for depth in 1..=segments.len() {
        let prefix = &segments[..depth];
        let url = build_remote_url(base_url, prefix)?;
        let dir_url = if url.ends_with('/') {
            url
        } else {
            format!("{url}/")
        };

        let resp = apply_auth(
            client
                .request(method_mkcol(), &dir_url)
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
            auth,
        )
        .send()
        .await
        .map_err(|e| AppError::Http(format!("WebDAV MKCOL 请求失败: {e}")))?;

        let status = resp.status();
        match status {
            s if s == StatusCode::CREATED || s.is_success() => {
                log::info!("[WebDAV] MKCOL ok: {}", redact_url(&dir_url));
            }
            s if s == StatusCode::METHOD_NOT_ALLOWED
                || s == StatusCode::CONFLICT
                || s.is_redirection() =>
            {
                // Directory likely already exists
            }
            _ => {
                return Err(webdav_status_error("MKCOL", status, &dir_url));
            }
        }
    }
    Ok(())
}

pub async fn put_bytes(
    client: &reqwest::Client,
    url: &str,
    auth: &WebDavAuth,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<(), AppError> {
    let resp = apply_auth(
        client
            .put(url)
            .header("Content-Type", content_type)
            .body(bytes)
            .timeout(Duration::from_secs(TRANSFER_TIMEOUT_SECS)),
        auth,
    )
    .send()
    .await
    .map_err(|e| AppError::Http(format!("WebDAV PUT 请求失败: {e}")))?;

    if resp.status().is_success() {
        return Ok(());
    }
    Err(webdav_status_error("PUT", resp.status(), url))
}

pub async fn get_bytes(
    client: &reqwest::Client,
    url: &str,
    auth: &WebDavAuth,
) -> Result<Option<Vec<u8>>, AppError> {
    let resp = apply_auth(
        client
            .get(url)
            .timeout(Duration::from_secs(TRANSFER_TIMEOUT_SECS)),
        auth,
    )
    .send()
    .await
    .map_err(|e| AppError::Http(format!("WebDAV GET 请求失败: {e}")))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(webdav_status_error("GET", resp.status(), url));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Http(format!("读取 WebDAV 响应失败: {e}")))?;
    Ok(Some(bytes.to_vec()))
}

pub async fn head_etag(
    client: &reqwest::Client,
    url: &str,
    auth: &WebDavAuth,
) -> Result<Option<(bool, Option<String>)>, AppError> {
    let resp = apply_auth(
        client
            .head(url)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
        auth,
    )
    .send()
    .await
    .map_err(|e| AppError::Http(format!("WebDAV HEAD 请求失败: {e}")))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(webdav_status_error("HEAD", resp.status(), url));
    }
    let last_modified = resp
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    Ok(Some((true, last_modified)))
}

pub fn is_jianguoyun(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
        .map(|host| host.contains("jianguoyun.com") || host.contains("nutstore"))
        .unwrap_or(false)
}

pub fn webdav_status_error(op: &str, status: StatusCode, url: &str) -> AppError {
    let safe_url = redact_url(url);
    let mut msg = format!("WebDAV {op} 失败: {status} ({safe_url})");

    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        if is_jianguoyun(url) {
            msg.push_str("。坚果云请使用「第三方应用密码」，并确认地址指向 /dav/ 下的目录。");
        } else {
            msg.push_str("。请检查 WebDAV 用户名、密码及目录读写权限。");
        }
    } else if is_jianguoyun(url) && (status == StatusCode::NOT_FOUND || status.is_redirection()) {
        msg.push_str("。坚果云常见原因：地址不在 /dav/ 可写目录下。");
    }

    AppError::Internal(msg)
}

fn redact_url(raw: &str) -> String {
    match Url::parse(raw) {
        Ok(mut parsed) => {
            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);
            parsed.to_string()
        }
        Err(_) => raw.split('?').next().unwrap_or(raw).to_string(),
    }
}
