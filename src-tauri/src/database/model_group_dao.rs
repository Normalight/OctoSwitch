use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::domain::model_group::{ModelGroup, NewModelGroup};
use crate::domain::model_slug;

pub fn list(conn: &Connection) -> Result<Vec<ModelGroup>, String> {
    let mut stmt = conn
        .prepare("SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups ORDER BY sort_order ASC, alias")
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |row| {
            Ok(ModelGroup {
                id: row.get(0)?,
                alias: row.get(1)?,
                active_binding_id: row.get(2)?,
                is_enabled: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn get_by_alias_ci(conn: &Connection, alias: &str) -> Result<Option<ModelGroup>, String> {
    let needle = alias.trim();
    if needle.is_empty() {
        return Ok(None);
    }
    let mut stmt = conn
        .prepare("SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups WHERE lower(alias) = lower(?1)")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([needle]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(ModelGroup {
            id: row.get(0).map_err(|e| e.to_string())?,
            alias: row.get(1).map_err(|e| e.to_string())?,
            active_binding_id: row.get(2).map_err(|e| e.to_string())?,
            is_enabled: row.get::<_, i64>(3).map_err(|e| e.to_string())? != 0,
            sort_order: row.get(4).map_err(|e| e.to_string())?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ModelGroup>, String> {
    let mut stmt = conn
        .prepare("SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups WHERE id=?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(ModelGroup {
            id: row.get(0).map_err(|e| e.to_string())?,
            alias: row.get(1).map_err(|e| e.to_string())?,
            active_binding_id: row.get(2).map_err(|e| e.to_string())?,
            is_enabled: row.get::<_, i64>(3).map_err(|e| e.to_string())? != 0,
            sort_order: row.get(4).map_err(|e| e.to_string())?,
        }))
    } else {
        Ok(None)
    }
}

/// 是否存在与 `alias`（忽略大小写）冲突的分组
pub fn alias_conflicts_with_model_name(
    conn: &Connection,
    alias: &str,
    exclude_group_id: Option<&str>,
) -> Result<bool, String> {
    let needle = alias.trim();
    if needle.is_empty() {
        return Ok(true);
    }
    let mut stmt = conn
        .prepare("SELECT id FROM model_groups WHERE lower(alias) = lower(?1)")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([needle]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let gid: String = row.get(0).map_err(|e| e.to_string())?;
        if exclude_group_id != Some(gid.as_str()) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn create(conn: &Connection, input: NewModelGroup) -> Result<ModelGroup, String> {
    let alias = input.alias.trim().to_string();
    if alias.is_empty() {
        return Err("分组别名不能为空".to_string());
    }
    model_slug::validate_no_slash(&alias, "分组别名")?;
    if alias_conflicts_with_model_name(conn, &alias, None)? {
        return Err(format!(
            "别名「{alias}」与已有分组或其它逻辑模型名冲突（忽略大小写）"
        ));
    }
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM model_groups", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO model_groups (id,alias,active_binding_id,sort_order) VALUES (?1,?2,NULL,?3)",
        params![id, alias, next_sort_order],
    )
    .map_err(|e| e.to_string())?;
    Ok(ModelGroup {
        id,
        alias,
        active_binding_id: None,
        is_enabled: true,
        sort_order: next_sort_order,
    })
}

pub fn update_partial(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
    ) -> Result<ModelGroup, String> {
    let current = get_by_id(conn, id)?
        .ok_or_else(|| "model group not found".to_string())?;
    let mut next = current.clone();

    if let Some(v) = patch.get("alias").and_then(|v| v.as_str()) {
        let a = v.trim().to_string();
        if a.is_empty() {
            return Err("分组别名不能为空".to_string());
        }
        model_slug::validate_no_slash(&a, "分组别名")?;
        if alias_conflicts_with_model_name(conn, &a, Some(id))? {
            return Err(format!(
                "别名「{a}」与已有分组或其它逻辑模型名冲突（忽略大小写）"
            ));
        }
        next.alias = a;
    }

    if let Some(v) = patch.get("is_enabled").and_then(|v| v.as_bool()) {
        next.is_enabled = v;
    }
    if let Some(v) = patch.get("sort_order").and_then(|v| v.as_i64()) {
        next.sort_order = v;
    }

    conn.execute(
        "UPDATE model_groups SET alias=?2, is_enabled=?3, sort_order=?4 WHERE id=?1",
        params![next.id, next.alias, next.is_enabled as i64, next.sort_order],
    )
    .map_err(|e| e.to_string())?;

    Ok(next)
}

pub fn set_active_binding(
    conn: &Connection,
    group_id: &str,
    binding_id: Option<&str>,
) -> Result<ModelGroup, String> {
    let mut g = get_by_id(conn, group_id)?
        .ok_or_else(|| "未找到模型分组".to_string())?;

    match binding_id {
        None => {
            g.active_binding_id = None;
        }
        Some(bid) => {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
                    params![group_id, bid],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("该绑定不属于此分组".to_string());
            }
            g.active_binding_id = Some(bid.to_string());
        }
    }

    conn.execute(
        "UPDATE model_groups SET active_binding_id=?2 WHERE id=?1",
        params![g.id, g.active_binding_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(g)
}

/// 导入配置时保留 id
pub fn insert_with_id(conn: &Connection, g: &ModelGroup) -> Result<(), String> {
    model_slug::validate_no_slash(g.alias.trim(), "分组别名")?;
    conn.execute(
        "INSERT INTO model_groups (id,alias,active_binding_id,sort_order) VALUES (?1,?2,?3,?4)",
        params![g.id, g.alias, g.active_binding_id, g.sort_order],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM model_group_members WHERE group_id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    let n = conn
        .execute("DELETE FROM model_groups WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("未找到该模型分组".to_string());
    }
    Ok(())
}

/// 某绑定被删除时，若分组正指向它则清空活动成员
pub fn clear_active_if_points_to(conn: &Connection, binding_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE model_groups SET active_binding_id = NULL WHERE active_binding_id = ?1",
        [binding_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
