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
    (3, include_str!("migrations/003_add_cache_tokens.sql")),
    (4, include_str!("migrations/004_add_auth_mode.sql")),
    (
        5,
        include_str!("migrations/005_add_task_route_preferences.sql"),
    ),
    (
        6,
        include_str!("migrations/006_add_delegate_agent_kind.sql"),
    ),
    (
        7,
        include_str!("migrations/007_add_delegate_model.sql"),
    ),
    (
        8,
        include_str!("migrations/008_drop_total_cost.sql"),
    ),
    (
        9,
        include_str!("migrations/009_metrics_snapshots.sql"),
    ),
    (
        10,
        include_str!("migrations/010_add_metrics_snapshots_index.sql"),
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
        2 => conn
            .execute_batch(sql)
            .map_err(|e| format!("migration v{version} failed: {e}")),
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
        5 => conn
            .execute_batch(sql)
            .map_err(|e| format!("migration v{version} failed: {e}")),
        6 => {
            if !table_has_column(conn, "task_route_preferences", "delegate_agent_kind")? {
                conn.execute_batch(sql)
                    .map_err(|e| format!("migration v{version} failed: {e}"))?;
            }
            Ok(())
        }
        7 => {
            if !table_has_column(conn, "task_route_preferences", "delegate_model")? {
                conn.execute_batch(sql)
                    .map_err(|e| format!("migration v{version} failed: {e}"))?;
            }
            Ok(())
        }
        8 => {
            if table_has_column(conn, "request_logs", "total_cost")?
                || table_has_column(conn, "model_bindings", "input_price_per_1m")?
                || table_has_column(conn, "model_bindings", "output_price_per_1m")?
            {
                conn.execute_batch(sql)
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
        if version > current || migration_is_missing(conn, version)? {
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

/// Check whether a migration was truly applied by verifying its expected side-effects.
/// This catches cases where schema_version was bumped by an older init_schema
/// but the actual migration hadn't been written yet.
fn migration_is_missing(conn: &Connection, version: i64) -> Result<bool, String> {
    match version {
        3 => Ok(!table_has_column(conn, "request_logs", "cache_creation_input_tokens")?
            || !table_has_column(conn, "request_logs", "cache_read_input_tokens")?),
        4 => Ok(!table_has_column(conn, "providers", "auth_mode")?),
        6 => Ok(!table_has_column(conn, "task_route_preferences", "delegate_agent_kind")?),
        7 => Ok(!table_has_column(conn, "task_route_preferences", "delegate_model")?),
        9 => {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='metrics_snapshots')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            Ok(!exists)
        }
        10 => {
            if !migration_is_missing(conn, 9)? {
                // Only check for the index if the metrics_snapshots table exists
                let index_exists: bool = conn
                    .query_row(
                        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_metrics_snapshots_time')",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                Ok(!index_exists)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}
