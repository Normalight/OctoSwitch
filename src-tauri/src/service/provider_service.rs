use rusqlite::Connection;

use crate::domain::error::AppError;
use crate::domain::provider::{NewProvider, Provider};
use crate::database::provider_dao;

pub fn list_providers(conn: &Connection) -> Result<Vec<Provider>, AppError> {
    provider_dao::list(conn).map_err(|e| AppError::Internal(e))
}

pub fn create_provider(conn: &Connection, input: NewProvider) -> Result<Provider, AppError> {
    provider_dao::create(conn, input).map_err(|e| AppError::Internal(e))
}

pub fn update_provider(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<Provider, AppError> {
    provider_dao::update_partial(conn, id, patch).map_err(|e| match e.to_lowercase().as_str() {
        msg if msg.contains("not found") => AppError::ProviderNotFound { provider_id: id.to_string() },
        _ => AppError::Internal(e),
    })
}

pub fn delete_provider(conn: &Connection, id: &str) -> Result<(), AppError> {
    provider_dao::delete(conn, id).map_err(|e| {
        if e.contains("未找到") {
            AppError::ProviderNotFound { provider_id: id.to_string() }
        } else {
            AppError::Internal(e)
        }
    })
}

pub fn get_provider(conn: &Connection, id: &str) -> Result<Option<Provider>, AppError> {
    provider_dao::get_by_id(conn, id).map_err(|e| AppError::Internal(e))
}
