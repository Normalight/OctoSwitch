use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotAccount {
    pub id: i64,
    pub provider_id: String,
    pub github_user_id: Option<i64>,
    pub github_login: String,
    pub avatar_url: Option<String>,
    pub github_token: Option<String>,
    pub copilot_token: Option<String>,
    pub token_expires_at: Option<String>,
    pub account_type: String,
    pub api_endpoint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 完整的 GitHub 用户信息（用于多账号识别）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: i64,
    pub avatar_url: String,
}

/// 类型化的 Copilot 认证错误
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum CopilotAuthError {
    #[error("授权尚未完成，请在浏览器中完成授权")]
    AuthorizationPending,
    #[error("授权已被拒绝")]
    AccessDenied,
    #[error("设备码已过期，请重新开始授权")]
    ExpiredToken,
    #[error("GitHub Token 无效: {0}")]
    GitHubTokenInvalid(String),
    #[error("Copilot Token 获取失败: {0}")]
    CopilotTokenFetchFailed(String),
    #[error("未找到 Copilot 订阅")]
    NoCopilotSubscription,
    #[error("未找到账号: {0}")]
    AccountNotFound(String),
    #[error("网络请求失败: {0}")]
    NetworkError(String),
    #[error("解析失败: {0}")]
    ParseError(String),
    #[error("内部错误: {0}")]
    Internal(String),
}
