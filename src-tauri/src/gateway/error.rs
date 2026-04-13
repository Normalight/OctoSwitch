//! 网关转发错误类型：HTTP 状态码、JSON `code`，以及 `INVALID_MODEL_SPEC` 的 `subcode`。

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

use crate::domain::error::AppError;

#[derive(Debug)]
pub enum ForwardRequestError {
    /// 未找到绑定或模型名未配置
    ModelNotBound { model: String },
    /// 绑定已禁用
    ModelBindingDisabled { model: String },
    /// 关联的供应商不存在
    ProviderNotFound { provider_id: String },
    /// 供应商已禁用
    ProviderDisabled { name: String },
    /// 上游网络或 HTTP 层错误
    Upstream(String),
    /// 客户端 `model` 格式非法（如绑定名中含 `/`、分段为空）
    InvalidModelSpec {
        subcode: &'static str,
        message: String,
    },
}

impl From<AppError> for ForwardRequestError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::ModelNotBound { model } => ForwardRequestError::ModelNotBound { model },
            AppError::ModelBindingDisabled { model } => {
                ForwardRequestError::ModelBindingDisabled { model }
            }
            AppError::ProviderNotFound { provider_id } => {
                ForwardRequestError::ProviderNotFound { provider_id }
            }
            AppError::InvalidModelSpec { subcode, message } => {
                ForwardRequestError::InvalidModelSpec { subcode, message }
            }
            _ => ForwardRequestError::Upstream(e.to_string()),
        }
    }
}

impl ForwardRequestError {
    pub fn user_message(&self) -> String {
        match self {
            ForwardRequestError::ModelNotBound { model } => {
                format!("Model '{model}' is not bound to any upstream provider. Please configure a model binding first.")
            }
            ForwardRequestError::ModelBindingDisabled { model } => {
                format!("Binding for model '{model}' is disabled. Please enable the binding or use a different model name.")
            }
            ForwardRequestError::ProviderNotFound { provider_id } => {
                format!(
                    "Bound provider not found (id={provider_id}). Please check your configuration."
                )
            }
            ForwardRequestError::ProviderDisabled { name } => {
                format!("Provider '{name}' is disabled and cannot forward requests.")
            }
            ForwardRequestError::Upstream(msg) => msg.clone(),
            ForwardRequestError::InvalidModelSpec { message, .. } => message.clone(),
        }
    }

    pub fn http_status(&self) -> StatusCode {
        let code = match self {
            ForwardRequestError::ModelNotBound { .. }
            | ForwardRequestError::ModelBindingDisabled { .. } => 404u16,
            ForwardRequestError::ProviderDisabled { .. } => 403u16,
            ForwardRequestError::InvalidModelSpec { .. } => 400u16,
            ForwardRequestError::ProviderNotFound { .. } | ForwardRequestError::Upstream(_) => {
                502u16
            }
        };
        StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY)
    }

    pub fn response_json(&self) -> Value {
        let code_str = match self {
            ForwardRequestError::ModelNotBound { .. }
            | ForwardRequestError::ModelBindingDisabled { .. } => "MODEL_NOT_AVAILABLE",
            ForwardRequestError::ProviderDisabled { .. } => "REQUEST_BLOCKED",
            ForwardRequestError::InvalidModelSpec { .. } => "INVALID_MODEL_SPEC",
            ForwardRequestError::ProviderNotFound { .. } | ForwardRequestError::Upstream(_) => {
                "UPSTREAM_OR_CONFIG_ERROR"
            }
        };
        match self {
            ForwardRequestError::InvalidModelSpec { subcode, message } => json!({
                "error": message,
                "code": code_str,
                "subcode": subcode,
            }),
            _ => json!({
                "error": self.user_message(),
                "code": code_str,
            }),
        }
    }

    pub fn into_axum_response(self) -> (StatusCode, Json<Value>) {
        if log::log_enabled!(log::Level::Debug) {
            match &self {
                ForwardRequestError::InvalidModelSpec { subcode, message } => {
                    log::debug!(
                        target: "octoswitch::gateway",
                        "model spec rejected: http_code={} code=INVALID_MODEL_SPEC subcode={} message={}",
                        self.http_status().as_u16(),
                        subcode,
                        message
                    );
                }
                other => {
                    log::debug!(
                        target: "octoswitch::gateway",
                        "gateway error response: http_code={} message={}",
                        other.http_status().as_u16(),
                        other.user_message()
                    );
                }
            }
        }
        (self.http_status(), Json(self.response_json()))
    }
}
