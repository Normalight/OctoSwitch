// src-tauri/src/repository/sqlite/migrations.rs

use rusqlite::Connection;

/// 初始建表 SQL（首次运行时执行）
pub const INITIAL_SCHEMA: &str = include_str!("migrations/001_initial_schema.sql");

/// Incremental migrations applied after the initial schema.
/// Each entry is (version, SQL). The runner applies any whose version > current schema_version.
const INCREMENTAL_MIGRATIONS: &[(i64, &str)] = &[
    (
        2,
        include_str!("migrations/002_add_provider_id_indexes.sql"),
    ),
    (
        3,
        include_str!("migrations/003_add_cache_tokens.sql"),
    ),
    (
        4,
        include_str!("migrations/004_add_auth_mode.sql"),
    ),
];

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let name: String = row.get(1).map_err(|e| e.to_string())?;
        if name.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn apply_migration(conn: &Connection, version: i64, sql: &str) -> Result<(), String> {
    match version {
        3 => {
            if !table_has_column(conn, "request_logs", "cache_creation_input_tokens")? {
                conn.execute(
                    "ALTER TABLE request_logs ADD COLUMN cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| format!("migration v{version} failed: {e}"))?;
            }
            if !table_has_column(conn, "request_logs", "cache_read_input_tokens")? {
                conn.execute(
                    "ALTER TABLE request_logs ADD COLUMN cache_read_input_tokens INTEGER NOT NULL DEFAULT 0",
                    [],
                )
                .map_err(|e| format!("migration v{version} failed: {e}"))?;
            }
            Ok(())
        }
        4 => {
            if !table_has_column(conn, "providers", "auth_mode")? {
                conn.execute(
                    "ALTER TABLE providers ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'bearer'",
                    [],
                )
                .map_err(|e| format!("migration v{version} failed: {e}"))?;
            }
            Ok(())
        }
        _ => conn
            .execute_batch(sql)
            .map_err(|e| format!("migration v{version} failed: {e}")),
    }
}

/// Apply any pending incremental migrations. Called after `init_schema`.
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for &(version, sql) in INCREMENTAL_MIGRATIONS {
        if version > current {
            apply_migration(conn, version, sql)?;
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
                rusqlite::params![version],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
