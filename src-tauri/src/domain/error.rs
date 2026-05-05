use crate::database::DaoError;

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AppError {
    #[error("Model '{model}' is not bound to any upstream provider")]
    ModelNotBound { model: String },

    /// Invalid `model` format; `subcode` matches the HTTP JSON `subcode` for diagnostics and i18n.
    #[error("[{subcode}] {message}")]
    InvalidModelSpec {
        subcode: &'static str,
        message: String,
    },

    #[error("Binding for model '{model}' is disabled")]
    ModelBindingDisabled { model: String },

    #[error("Model group '{alias}' is disabled")]
    ModelGroupDisabled { alias: String },

    #[error("Provider not found (id={provider_id})")]
    ProviderNotFound { provider_id: String },

    #[error("Database error: {0}")]
    Database(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("JSON parse error: {0}")]
    Json(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Copilot authentication error: {0}")]
    #[allow(dead_code)]
    CopilotAuth(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<DaoError> for AppError {
    fn from(e: DaoError) -> Self {
        match e {
            DaoError::NotFound { entity: "provider", id } => AppError::ProviderNotFound {
                provider_id: id,
            },
            DaoError::NotFound { .. } => AppError::Internal(e.to_string()),
            DaoError::AlreadyExists { .. } => AppError::Internal(e.to_string()),
            DaoError::Validation { .. } => AppError::Internal(e.to_string()),
            DaoError::Sql(err) => AppError::Database(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_not_bound_displays_model_name() {
        let e = AppError::ModelNotBound { model: "gpt-4".into() };
        assert!(e.to_string().contains("gpt-4"));
    }

    #[test]
    fn model_binding_disabled_displays_model_name() {
        let e = AppError::ModelBindingDisabled { model: "claude".into() };
        assert!(e.to_string().contains("claude"));
    }

    #[test]
    fn provider_not_found_displays_id() {
        let e = AppError::ProviderNotFound { provider_id: "abc-123".into() };
        assert!(e.to_string().contains("abc-123"));
    }

    #[test]
    fn invalid_model_spec_contains_subcode_and_message() {
        let e = AppError::InvalidModelSpec {
            subcode: "MODEL_SPEC_EMPTY",
            message: "Model name cannot be empty.".into(),
        };
        let s = e.to_string();
        assert!(s.contains("MODEL_SPEC_EMPTY"));
        assert!(s.contains("Model name cannot be empty"));
    }

    #[test]
    fn dao_error_provider_not_found_converted() {
        let dao = DaoError::NotFound { entity: "provider", id: "xyz".into() };
        let app: AppError = dao.into();
        match app {
            AppError::ProviderNotFound { provider_id } => assert_eq!(provider_id, "xyz"),
            other => panic!("expected ProviderNotFound, got {other:?}"),
        }
    }

    #[test]
    fn dao_error_other_not_found_becomes_internal() {
        let dao = DaoError::NotFound { entity: "model_binding", id: "123".into() };
        let app: AppError = dao.into();
        match app {
            AppError::Internal(_) => {}
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn da_error_sql_becomes_database() {
        let dao = DaoError::Sql(rusqlite::Error::InvalidParameterName("bad".into()));
        let app: AppError = dao.into();
        assert!(matches!(app, AppError::Database(_)));
    }
}
