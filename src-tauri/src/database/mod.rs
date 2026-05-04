// src-tauri/src/database/mod.rs
pub mod copilot_account_dao;
pub mod model_binding_dao;
pub mod model_group_dao;
pub mod model_group_member_dao;
pub mod provider_dao;
pub mod task_route_preference_dao;

/// Convert a boolean to i64 (1/0) for SQLite storage.
pub(crate) fn bool_to_i64(v: bool) -> i64 {
    if v {
        1
    } else {
        0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaoError {
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("{entity} already exists: {id}")]
    AlreadyExists { entity: &'static str, id: String },

    #[error("validation: {field} — {message}")]
    Validation { field: &'static str, message: String },

    #[error("{0}")]
    Sql(#[from] rusqlite::Error),
}

use rusqlite::Connection;

const LATEST_SCHEMA_VERSION: i64 = 7;

pub fn init_schema(conn: &mut Connection) -> Result<(), String> {
    log::info!(
        "[{}] initializing database schema",
        crate::log_codes::DB_INIT
    );

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;

    // 只在首次（providers 表不存在）时执行完整建表 SQL
    let needs_init: bool = conn
        .query_row(
            "SELECT NOT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='providers')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(true);

    if needs_init {
        let sql = crate::repository::sqlite::migrations::INITIAL_SCHEMA;
        conn.execute_batch(sql).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            rusqlite::params![LATEST_SCHEMA_VERSION],
        )
        .map_err(|e| e.to_string())?;
    }

    // Apply any pending incremental migrations
    log::debug!(
        "[{}] applying incremental migrations",
        crate::log_codes::DB_MIGRATE
    );
    crate::repository::sqlite::migrations::run_migrations(conn)?;

    // 清理旧迁移遗留的触发器（如引用 providers_old 的触发器）
    cleanup_orphan_triggers(conn);

    Ok(())
}

fn cleanup_orphan_triggers(conn: &Connection) {
    let stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND sql LIKE '%_old%'");
    if let Ok(mut stmt) = stmt {
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .ok()
            .into_iter()
            .flat_map(|iter| iter.filter_map(|r| r.ok()).collect::<Vec<_>>())
            .collect();
        for name in names {
            // Validate trigger name is a valid SQL identifier
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let _ = conn.execute(&format!("DROP TRIGGER IF EXISTS {}", name), []);
            }
        }
    }
}

/// 清除所有用户数据，恢复到初始状态（保留表结构）
pub fn clear_all_data(conn: &Connection) -> Result<(), String> {
    let tables = [
        "model_group_members",
        "model_bindings",
        "task_route_preferences",
        "model_groups",
        "providers",
        "copilot_accounts",
        "request_logs",
    ];
    let mut statements = String::new();
    for table in &tables {
        statements.push_str(&format!("DELETE FROM {};\n", table));
    }
    statements.push_str("DELETE FROM schema_version;\n");
    statements.push_str(&format!(
        "INSERT INTO schema_version (version) VALUES ({});\n",
        LATEST_SCHEMA_VERSION
    ));
    conn.execute_batch(&statements).map_err(|e| e.to_string())?;
    Ok(())
}
