use crate::database::model_binding_dao;
use crate::domain::provider::{NewProvider, Provider};
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use super::bool_to_i64;

const PROVIDER_COLUMNS: &str = "id,name,base_url,api_key_ref,timeout_ms,max_retries,is_enabled,sort_order,api_format,auth_mode FROM providers";

pub fn list(conn: &Connection) -> Result<Vec<Provider>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {PROVIDER_COLUMNS} ORDER BY sort_order ASC, updated_at DESC"
        ))
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                base_url: row.get(2)?,
                api_key_ref: row.get(3)?,
                timeout_ms: row.get(4)?,
                max_retries: row.get(5)?,
                is_enabled: row.get::<_, i64>(6)? == 1,
                sort_order: row.get(7)?,
                api_format: row.get(8).ok(),
                auth_mode: row.get(9).unwrap_or_else(|_| "bearer".to_string()),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for p in iter {
        out.push(p.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 导入/恢复配置时使用，保留导出 JSON 中的 id。
pub fn insert_with_id(conn: &Connection, p: &Provider) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO providers (id,name,base_url,api_key_ref,timeout_ms,max_retries,is_enabled,sort_order,api_format,auth_mode,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            p.id,
            p.name,
            p.base_url,
            p.api_key_ref,
            p.timeout_ms,
            p.max_retries,
            bool_to_i64(p.is_enabled),
            p.sort_order,
            p.api_format,
            p.auth_mode,
            now,
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create(conn: &Connection, input: NewProvider) -> Result<Provider, String> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM providers",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO providers (id,name,base_url,api_key_ref,timeout_ms,max_retries,is_enabled,sort_order,api_format,auth_mode,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            id,
            input.name,
            input.base_url,
            input.api_key_ref,
            input.timeout_ms,
            input.max_retries,
            bool_to_i64(input.is_enabled),
            next_sort_order,
            input.api_format,
            input.auth_mode,
            now,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(Provider {
        id,
        name: input.name,
        base_url: input.base_url,
        api_key_ref: input.api_key_ref,
        timeout_ms: input.timeout_ms,
        max_retries: input.max_retries,
        is_enabled: input.is_enabled,
        sort_order: next_sort_order,
        api_format: input.api_format,
        auth_mode: input.auth_mode,
    })
}

pub fn update_partial(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<Provider, String> {
    let current = get_by_id(conn, id)?.ok_or_else(|| "provider not found".to_string())?;
    let mut next = current.clone();

    if let Some(v) = patch.get("name").and_then(|v| v.as_str()) {
        next.name = v.to_string();
    }
    if let Some(v) = patch.get("base_url").and_then(|v| v.as_str()) {
        next.base_url = v.to_string();
    }
    if let Some(v) = patch.get("api_key_ref").and_then(|v| v.as_str()) {
        next.api_key_ref = v.to_string();
    }
    if let Some(v) = patch.get("timeout_ms").and_then(|v| v.as_i64()) {
        next.timeout_ms = v;
    }
    if let Some(v) = patch.get("max_retries").and_then(|v| v.as_i64()) {
        next.max_retries = v;
    }
    if let Some(v) = patch.get("is_enabled").and_then(|v| v.as_bool()) {
        next.is_enabled = v;
    }
    if let Some(v) = patch.get("sort_order").and_then(|v| v.as_i64()) {
        next.sort_order = v;
    }
    if let Some(v) = patch.get("api_format") {
        next.api_format = v.as_str().map(String::from);
    }
    if let Some(v) = patch.get("auth_mode").and_then(|v| v.as_str()) {
        next.auth_mode = v.to_string();
    }

    conn.execute(
        "UPDATE providers SET name=?2,base_url=?3,api_key_ref=?4,timeout_ms=?5,max_retries=?6,is_enabled=?7,sort_order=?8,api_format=?9,auth_mode=?10,updated_at=?11 WHERE id=?1",
        params![
            next.id,
            next.name,
            next.base_url,
            next.api_key_ref,
            next.timeout_ms,
            next.max_retries,
            bool_to_i64(next.is_enabled),
            next.sort_order,
            next.api_format,
            next.auth_mode,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(next)
}

fn row_to_provider(row: &rusqlite::Row) -> Result<Provider, rusqlite::Error> {
    Ok(Provider {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        api_key_ref: row.get(3)?,
        timeout_ms: row.get(4)?,
        max_retries: row.get(5)?,
        is_enabled: row.get::<_, i64>(6)? == 1,
        sort_order: row.get(7)?,
        api_format: row.get(8).ok(),
        auth_mode: row.get(9).unwrap_or_else(|_| "bearer".to_string()),
    })
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Provider>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {PROVIDER_COLUMNS} WHERE id=?1"))
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query([id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(row_to_provider(row).map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

/// 删除供应商及其所有模型绑定、分组成员关系
pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    // 先删除该供应商下的所有模型绑定
    let binding_ids: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM model_bindings WHERE provider_id=?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([id]).map_err(|e| e.to_string())?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            ids.push(row.get(0).map_err(|e| e.to_string())?);
        }
        ids
    };
    for bid in &binding_ids {
        model_binding_dao::delete(conn, bid)?;
    }
    // 删除供应商
    let n = conn
        .execute("DELETE FROM providers WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("未找到该供应商".to_string());
    }
    Ok(())
}
