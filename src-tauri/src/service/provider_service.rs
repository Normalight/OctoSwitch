use rusqlite::Connection;

use crate::database::provider_dao;
use crate::domain::error::AppError;
use crate::domain::provider::{NewProvider, Provider};

pub fn list_providers(conn: &Connection) -> Result<Vec<Provider>, AppError> {
    Ok(provider_dao::list(conn)?)
}

pub fn create_provider(conn: &Connection, input: NewProvider) -> Result<Provider, AppError> {
    Ok(provider_dao::create(conn, input)?)
}

pub fn update_provider(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<Provider, AppError> {
    Ok(provider_dao::update_partial(conn, id, patch)?)
}

pub fn delete_provider(conn: &Connection, id: &str) -> Result<(), AppError> {
    Ok(provider_dao::delete(conn, id)?)
}

pub fn get_provider(conn: &Connection, id: &str) -> Result<Option<Provider>, AppError> {
    Ok(provider_dao::get_by_id(conn, id)?)
}
