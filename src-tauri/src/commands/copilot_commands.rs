use std::time::Duration;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::database::copilot_account_dao;
use crate::domain::copilot_account::CopilotAccount;
use crate::domain::error::AppError;
use crate::services::copilot_auth::{self, DeviceCodeResponse};
use crate::state::AppState;

#[tauri::command]
pub fn open_external_url(app: AppHandle, url: String) -> Result<(), AppError> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err(AppError::Internal("Only http(s) URLs are allowed".into()));
    }

    app.opener()
        .open_url(trimmed, None::<String>)
        .map_err(|e| AppError::Internal(format!("failed to open browser: {e}")))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CopilotAccountStatus {
    pub id: i64,
    pub provider_id: String,
    pub github_login: String,
    pub avatar_url: Option<String>,
    pub account_type: String,
    pub authenticated: bool,
    pub token_expires_at: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CopilotStatus {
    pub authenticated: bool,
    pub pending: bool,
    pub account_type: Option<String>,
    pub account_login: Option<String>,
    pub has_token: bool,
    pub token_expires_at: Option<String>,
}

impl CopilotStatus {
    fn not_authenticated() -> Self {
        Self {
            authenticated: false,
            pending: false,
            account_type: None,
            account_login: None,
            has_token: false,
            token_expires_at: None,
        }
    }
}

#[tauri::command]
pub async fn start_copilot_auth() -> Result<DeviceCodeResponse, AppError> {
    copilot_auth::request_device_code()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn complete_copilot_auth(
    state: State<'_, AppState>,
    device_code: String,
    provider_id: String,
) -> Result<CopilotStatus, AppError> {
    // 1. Short polling
    let github_token =
        match copilot_auth::poll_access_token_with_timeout(&device_code, Duration::from_secs(2))
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
        {
            Some(token) => token,
            None => {
                return Ok(CopilotStatus {
                    pending: true,
                    ..CopilotStatus::not_authenticated()
                });
            }
        };

    // 2. Fetch GitHub user info
    let user = copilot_auth::fetch_github_user(&github_token)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 3. Discover API endpoint (enterprise support)
    let api_endpoint = copilot_auth::discover_api_endpoint(&github_token)
        .await
        .ok();

    // 4. Get Copilot token
    let copilot_resp = copilot_auth::fetch_copilot_token(&github_token, api_endpoint.as_deref())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 5. Save to copilot_accounts + create provider in transaction
    let now = chrono::Utc::now().to_rfc3339();
    let account = CopilotAccount {
        id: 0,
        provider_id: provider_id.clone(),
        github_user_id: Some(user.id),
        github_login: user.login.clone(),
        avatar_url: Some(user.avatar_url.clone()),
        github_token: Some(github_token),
        copilot_token: Some(copilot_resp.token.clone()),
        token_expires_at: Some(copilot_resp.expires_at.to_string()),
        account_type: copilot_resp
            .account_type
            .clone()
            .unwrap_or_else(|| "individual".to_string()),
        api_endpoint,
        created_at: now.clone(),
        updated_at: now,
    };

    {
        let mut conn = state.db.get().map_err(|e| AppError::from(e.to_string()))?;
        let tx = conn.transaction()?;
        // 先确保 provider 存在，避免 copilot_accounts.provider_id 外键失败
        let existing = crate::database::provider_dao::get_by_id(&tx, &provider_id)?;
        if existing.is_none() {
            crate::database::provider_dao::insert_with_id(
                &tx,
                &crate::domain::provider::Provider {
                    id: provider_id.clone(),
                    // providers.name 是唯一键，使用 GitHub 用户名即可
                    name: user.login.clone(),
                    base_url: account
                        .api_endpoint
                        .clone()
                        .unwrap_or_else(|| "https://api.githubcopilot.com".to_string()),
                    api_key_ref: String::new(),
                    timeout_ms: 60000,
                    max_retries: 0,
                    sort_order: 0,
                    is_enabled: true,
                    api_format: None,
                    auth_mode: "bearer".to_string(),
                },
            )?;
        }

        // 同 provider 允许重复授权时覆盖更新，避免 UNIQUE(provider_id) 报错
        if let Some(existing_acc) =
            copilot_account_dao::get_by_provider(&tx, &provider_id)?
        {
            let mut updated = account.clone();
            updated.id = existing_acc.id;
            copilot_account_dao::update(&tx, &updated)?;
        } else {
            copilot_account_dao::insert(&tx, &account)?;
        }

        tx.commit()?;
    }

    Ok(CopilotStatus {
        authenticated: true,
        pending: false,
        account_type: Some(account.account_type),
        account_login: Some(account.github_login),
        has_token: account.copilot_token.is_some(),
        token_expires_at: account.token_expires_at,
    })
}

#[tauri::command]
pub async fn get_copilot_status(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<CopilotStatus, AppError> {
    let pid = provider_id.as_deref().unwrap_or("copilot");
    let account = {
        let conn = state.db.get()?;
        copilot_account_dao::get_by_provider(&conn, pid)?
    };

    match account {
        Some(acc) if acc.github_token.is_some() => {
            let effective = match copilot_auth::ensure_copilot_token(&acc).await {
                Ok(updated) => {
                    if updated.copilot_token != acc.copilot_token
                        || updated.token_expires_at != acc.token_expires_at
                    {
                        let conn = state.db.get()?;
                        copilot_account_dao::update(&conn, &updated)?;
                    }
                    updated
                }
                Err(_) => {
                    return Ok(CopilotStatus::not_authenticated());
                }
            };

            Ok(CopilotStatus {
                authenticated: true,
                pending: false,
                account_type: Some(effective.account_type),
                account_login: Some(effective.github_login),
                has_token: effective.copilot_token.is_some(),
                token_expires_at: effective.token_expires_at,
            })
        }
        _ => Ok(CopilotStatus::not_authenticated()),
    }
}

/// 列出所有 Copilot 账号
#[tauri::command]
pub async fn list_copilot_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<CopilotAccountStatus>, AppError> {
    let accounts = {
        let conn = state.db.get()?;
        copilot_account_dao::list(&conn)?
    };

    // Parallel token refresh for all accounts
    let refresh_futures: Vec<_> = accounts
        .iter()
        .map(|acc| async {
            let is_valid = acc.github_token.is_some();
            if !is_valid {
                return (acc.id, false, acc.token_expires_at.clone());
            }
            match copilot_auth::ensure_copilot_token(acc).await {
                Ok(updated) => {
                    if updated.copilot_token != acc.copilot_token
                        || updated.token_expires_at != acc.token_expires_at
                    {
                        let conn = state.db.get().map_err(|e| e.to_string());
                        if let Ok(conn) = conn {
                            if let Err(e) = copilot_account_dao::update(&conn, &updated) {
                                log::warn!(
                                    "[{}] failed to update copilot account: {e}",
                                    crate::log_codes::COP_TOKEN_PERSIST
                                );
                            }
                        }
                    }
                    (acc.id, true, updated.token_expires_at.clone())
                }
                Err(_) => (acc.id, false, acc.token_expires_at.clone()),
            }
        })
        .collect();

    let refresh_results = futures_util::future::join_all(refresh_futures).await;

    let mut result = Vec::with_capacity(accounts.len());
    for (acc, (_id, authenticated, updated_expires)) in
        accounts.iter().zip(refresh_results)
    {
        result.push(CopilotAccountStatus {
            id: acc.id,
            provider_id: acc.provider_id.clone(),
            github_login: acc.github_login.clone(),
            avatar_url: acc.avatar_url.clone(),
            account_type: acc.account_type.clone(),
            authenticated,
            token_expires_at: updated_expires,
            error: if !authenticated {
                Some("未认证或无订阅".to_string())
            } else {
                None
            },
        });
    }

    Ok(result)
}

/// 删除 Copilot 账号及其关联的供应商
#[tauri::command]
pub async fn remove_copilot_account(
    state: State<'_, AppState>,
    account_id: i64,
) -> Result<(), AppError> {
    let mut conn = state.db.get()?;
    let account = copilot_account_dao::get_by_id(&conn, account_id)?
        .ok_or_else(|| AppError::Internal("未找到该 Copilot 账号".into()))?;

    let tx = conn.transaction()?;
    copilot_account_dao::delete(&tx, account_id)?;
    let _ = crate::database::provider_dao::delete(&tx, &account.provider_id);
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub async fn refresh_copilot_token(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<CopilotStatus, AppError> {
    let pid = provider_id.as_deref().unwrap_or("copilot");
    let acc = {
        let conn = state.db.get()?;
        copilot_account_dao::get_by_provider(&conn, pid)?
            .ok_or_else(|| AppError::Internal("Not authenticated".into()))?
    };

    let updated = copilot_auth::ensure_copilot_token(&acc)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let conn = state.db.get()?;
    copilot_account_dao::update(&conn, &updated)?;

    Ok(CopilotStatus {
        authenticated: true,
        pending: false,
        account_type: Some(updated.account_type),
        account_login: Some(updated.github_login),
        has_token: updated.copilot_token.is_some(),
        token_expires_at: updated.token_expires_at,
    })
}

#[tauri::command]
pub async fn revoke_copilot_auth(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<(), AppError> {
    let pid = provider_id.as_deref().unwrap_or("copilot");
    let conn = state.db.get()?;
    copilot_account_dao::delete_by_provider(&conn, pid)?;

    let _ = crate::database::provider_dao::delete(&conn, pid);
    Ok(())
}

/// 获取 Copilot 可用模型
#[tauri::command]
pub async fn get_copilot_models(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<Vec<String>, AppError> {
    let pid = provider_id.as_deref().unwrap_or("copilot");
    let account = {
        let conn = state.db.get()?;
        copilot_account_dao::get_by_provider(&conn, pid)?
            .ok_or_else(|| AppError::Internal("Not authenticated".into()))?
    };

    let updated = copilot_auth::ensure_copilot_token(&account)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if updated.copilot_token != account.copilot_token
        || updated.token_expires_at != account.token_expires_at
    {
        let conn = state.db.get()?;
        copilot_account_dao::update(&conn, &updated)?;
    }
    let jwt = updated
        .copilot_token
        .as_deref()
        .ok_or_else(|| AppError::Internal("Copilot token missing".into()))?;

    copilot_auth::fetch_copilot_models(jwt, updated.api_endpoint.as_deref())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// 获取 Copilot 用量信息
#[tauri::command]
pub async fn get_copilot_usage(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let pid = provider_id.as_deref().unwrap_or("copilot");
    let account = {
        let conn = state.db.get()?;
        copilot_account_dao::get_by_provider(&conn, pid)?
            .ok_or_else(|| AppError::Internal("Not authenticated".into()))?
    };

    let github_token = account
        .github_token
        .ok_or_else(|| AppError::Internal("Not authenticated".into()))?;

    copilot_auth::fetch_copilot_usage(&github_token, account.api_endpoint.as_deref())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}
