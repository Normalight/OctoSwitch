// Shared test utilities for the OctoSwitch Rust backend.
// This module is only compiled during `cargo test` via the `#[cfg(test)]` gate in main.rs.

use rusqlite::Connection;

use crate::domain::model_binding::{ModelBinding, NewModelBinding};
use crate::domain::model_group::{ModelGroup, NewModelGroup};
use crate::domain::provider::{NewProvider, Provider};

/// Create an in-memory SQLite database with the full schema (initial + migrations) applied.
pub fn test_db() -> Connection {
    let mut conn = Connection::open_in_memory().expect("Failed to create test DB");
    crate::database::init_schema(&mut conn).expect("Failed to initialise database schema");
    conn
}

// ---------------------------------------------------------------------------
// Provider factories
// ---------------------------------------------------------------------------

/// Create a default test provider in the database.
pub fn test_provider(conn: &Connection) -> Provider {
    test_provider_named(conn, "test-provider")
}

/// Create a named test provider in the database.
pub fn test_provider_named(conn: &Connection, name: &str) -> Provider {
    crate::database::provider_dao::create(
        conn,
        NewProvider {
            name: name.to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key_ref: "sk-test".to_string(),
            timeout_ms: 30000,
            max_retries: 2,
            is_enabled: true,
            api_format: Some("openai_chat".to_string()),
            auth_mode: "bearer".to_string(),
        },
    )
    .expect("create test provider")
}

// ---------------------------------------------------------------------------
// ModelBinding factories
// ---------------------------------------------------------------------------

/// Create a default test model binding for a given provider.
pub fn test_model_binding(conn: &Connection, provider_id: &str) -> ModelBinding {
    test_model_binding_named(conn, "test-model", provider_id)
}

/// Create a named test model binding for a given provider.
pub fn test_model_binding_named(
    conn: &Connection,
    model_name: &str,
    provider_id: &str,
) -> ModelBinding {
    crate::database::model_binding_dao::create(
        conn,
        NewModelBinding {
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            upstream_model_name: format!("up-{}", model_name),
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        },
    )
    .expect("create test binding")
}

// ---------------------------------------------------------------------------
// ModelGroup factories
// ---------------------------------------------------------------------------

/// Create a test model group with the given alias.
pub fn test_model_group(conn: &Connection, alias: &str) -> ModelGroup {
    crate::database::model_group_dao::create(
        conn,
        NewModelGroup {
            alias: alias.to_string(),
        },
    )
    .expect("create test group")
}
