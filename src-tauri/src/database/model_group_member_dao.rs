use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::database::model_group_dao;

/// 关联：分组与绑定多对多；若分组尚无活动成员，则将该绑定设为活动
pub fn add(conn: &Connection, group_id: &str, binding_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO model_group_members (group_id, binding_id) VALUES (?1, ?2)",
        params![group_id, binding_id],
    )
    .map_err(|e| e.to_string())?;
    let g = model_group_dao::get_by_id(conn, group_id)?
        .ok_or_else(|| "未找到模型分组".to_string())?;
    if g.active_binding_id.is_none() && is_member(conn, group_id, binding_id)? {
        model_group_dao::set_active_binding(conn, group_id, Some(binding_id))?;
    }
    Ok(())
}

pub fn remove(conn: &Connection, group_id: &str, binding_id: &str) -> Result<(), String> {
    conn
        .execute(
            "DELETE FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
            params![group_id, binding_id],
        )
        .map_err(|e| e.to_string())?;
    sync_active_after_remove(conn, group_id, binding_id)?;
    Ok(())
}

/// 删除成员后：若活动成员被移除，则改为剩余成员之一或清空
fn sync_active_after_remove(conn: &Connection, group_id: &str, removed_binding_id: &str) -> Result<(), String> {
    let g = model_group_dao::get_by_id(conn, group_id)?
        .ok_or_else(|| "group missing".to_string())?;
    if g.active_binding_id.as_deref() != Some(removed_binding_id) {
        return Ok(());
    }
    let remaining = list_binding_ids_for_group(conn, group_id)?;
    let next = remaining.first().map(|s| s.as_str());
    model_group_dao::set_active_binding(conn, group_id, next)?;
    Ok(())
}

pub fn list_binding_ids_for_group(conn: &Connection, group_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT binding_id FROM model_group_members WHERE group_id = ?1 ORDER BY binding_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([group_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn list_group_ids_for_binding(conn: &Connection, binding_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT group_id FROM model_group_members WHERE binding_id = ?1 ORDER BY group_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([binding_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn group_ids_map_for_all_bindings(conn: &Connection) -> Result<HashMap<String, Vec<String>>, String> {
    let mut stmt = conn
        .prepare("SELECT binding_id, group_id FROM model_group_members ORDER BY binding_id, group_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    for r in rows {
        let (bid, gid) = r.map_err(|e| e.to_string())?;
        m.entry(bid).or_default().push(gid);
    }
    Ok(m)
}

pub fn is_member(conn: &Connection, group_id: &str, binding_id: &str) -> Result<bool, String> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
            params![group_id, binding_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn delete_all_for_binding(conn: &Connection, binding_id: &str) -> Result<(), String> {
    let groups = list_group_ids_for_binding(conn, binding_id)?;
    conn.execute(
        "DELETE FROM model_group_members WHERE binding_id = ?1",
        [binding_id],
    )
    .map_err(|e| e.to_string())?;
    for gid in groups {
        let g = model_group_dao::get_by_id(conn, &gid)?;
        if let Some(g) = g {
            if g.active_binding_id.as_deref() == Some(binding_id) {
                let remaining = list_binding_ids_for_group(conn, &gid)?;
                let next = remaining.first().map(|s| s.as_str());
                model_group_dao::set_active_binding(conn, &gid, next)?;
            }
        }
    }
    Ok(())
}

/// 用于 `GET /v1/models`：每个分组成员对应一条 `(分组别名, 绑定路由名)`，按分组排序。
pub fn list_group_binding_pairs_for_catalog(
    conn: &Connection,
) -> Result<Vec<(String, String, bool, bool)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.alias, b.model_name, g.is_enabled, b.is_enabled \
             FROM model_group_members m \
             JOIN model_groups g ON g.id = m.group_id \
             JOIN model_bindings b ON b.id = m.binding_id \
             ORDER BY g.sort_order ASC, g.alias ASC, b.model_name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, i64>(3)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn export_pairs(conn: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT group_id, binding_id FROM model_group_members ORDER BY group_id, binding_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}
