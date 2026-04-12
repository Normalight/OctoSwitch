#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Model '{model}' is not bound to any upstream provider")]
    ModelNotBound { model: String },

    /// 非法的 `model` 格式；`subcode` 与 HTTP JSON `subcode` 一致，便于排障与 i18n
    #[error("[{subcode}] {message}")]
    InvalidModelSpec {
        subcode: &'static str,
        message: String,
    },

    #[error("Binding for model '{model}' is disabled")]
    ModelBindingDisabled { model: String },

    #[error("Provider not found (id={provider_id})")]
    ProviderNotFound { provider_id: String },

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Copilot authentication error: {0}")]
    #[allow(dead_code)]
    CopilotAuth(String),
}
