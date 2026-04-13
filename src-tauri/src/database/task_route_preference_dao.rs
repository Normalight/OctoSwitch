use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::domain::task_route_preference::{NewTaskRoutePreference, TaskRoutePreference};

fn normalize_non_empty(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} 不能为空"));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn ensure_unique_task_kind(conn: &Connection, task_kind: &str, except_id: Option<&str>) -> Result<(), String> {
    let existing_id = conn
        .query_row(
            "SELECT id
             FROM task_route_preferences
             WHERE lower(trim(task_kind)) = lower(trim(?1))
             LIMIT 1",
            [task_kind],
            |row| row.get::<_, String>(0),
        )
        .ok();

    if let Some(existing_id) = existing_id {
        if except_id.map(|id| id != existing_id).unwrap_or(true) {
            return Err(format!("task_kind `{task_kind}` 已存在，请直接编辑该偏好"));
        }
    }

    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<TaskRoutePreference>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_kind, target_group, target_member, delegate_model, prompt_template, is_enabled, sort_order
             FROM task_route_preferences
             ORDER BY sort_order ASC, task_kind ASC",
        )
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |row| {
            Ok(TaskRoutePreference {
                id: row.get(0)?,
                task_kind: row.get(1)?,
                target_group: row.get(2)?,
                target_member: row.get(3)?,
                delegate_model: row.get(4)?,
                prompt_template: row.get(5)?,
                is_enabled: row.get::<_, i64>(6)? != 0,
                sort_order: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<TaskRoutePreference>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task_kind, target_group, target_member, delegate_model, prompt_template, is_enabled, sort_order
             FROM task_route_preferences
             WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(TaskRoutePreference {
            id: row.get(0).map_err(|e| e.to_string())?,
            task_kind: row.get(1).map_err(|e| e.to_string())?,
            target_group: row.get(2).map_err(|e| e.to_string())?,
            target_member: row.get(3).map_err(|e| e.to_string())?,
            delegate_model: row.get(4).map_err(|e| e.to_string())?,
            prompt_template: row.get(5).map_err(|e| e.to_string())?,
            is_enabled: row.get::<_, i64>(6).map_err(|e| e.to_string())? != 0,
            sort_order: row.get(7).map_err(|e| e.to_string())?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create(
    conn: &Connection,
    input: NewTaskRoutePreference,
) -> Result<TaskRoutePreference, String> {
    let task_kind = normalize_non_empty(&input.task_kind, "task_kind")?;
    let target_group = normalize_non_empty(&input.target_group, "target_group")?;
    let prompt_template = normalize_optional(input.prompt_template);
    ensure_unique_task_kind(conn, &task_kind, None)?;
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM task_route_preferences",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO task_route_preferences (id, task_kind, target_group, target_member, delegate_agent_kind, delegate_model, prompt_template, is_enabled, sort_order)
         VALUES (?1, ?2, ?3, ?4, 'auto', ?5, ?6, ?7, ?8)",
        params![
            id,
            task_kind,
            target_group,
            Option::<String>::None,
            Option::<String>::None,
            prompt_template,
            input.is_enabled as i64,
            next_sort_order
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(TaskRoutePreference {
        id,
        task_kind,
        target_group,
        target_member: None,
        delegate_model: None,
        prompt_template,
        is_enabled: input.is_enabled,
        sort_order: next_sort_order,
    })
}

pub fn update_partial(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<TaskRoutePreference, String> {
    let current =
        get_by_id(conn, id)?.ok_or_else(|| "task route preference not found".to_string())?;
    let mut next = current.clone();

    if let Some(v) = patch.get("task_kind").and_then(|v| v.as_str()) {
        next.task_kind = normalize_non_empty(v, "task_kind")?;
    }
    if let Some(v) = patch.get("target_group").and_then(|v| v.as_str()) {
        next.target_group = normalize_non_empty(v, "target_group")?;
    }
    if let Some(v) = patch.get("target_member").and_then(|v| v.as_str()) {
        next.target_member = if v.is_empty() { None } else { Some(v.to_string()) };
    }
    if let Some(v) = patch.get("delegate_model").and_then(|v| v.as_str()) {
        next.delegate_model = if v.is_empty() { None } else { Some(v.to_string()) };
    }
    if patch.get("prompt_template").is_some() {
        next.prompt_template = patch
            .get("prompt_template")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    if let Some(v) = patch.get("is_enabled").and_then(|v| v.as_bool()) {
        next.is_enabled = v;
    }
    if let Some(v) = patch.get("sort_order").and_then(|v| v.as_i64()) {
        next.sort_order = v;
    }
    ensure_unique_task_kind(conn, &next.task_kind, Some(id))?;

    conn.execute(
        "UPDATE task_route_preferences
         SET task_kind = ?2, target_group = ?3, target_member = ?4, delegate_model = ?5, prompt_template = ?6, is_enabled = ?7, sort_order = ?8
         WHERE id = ?1",
        params![
            next.id,
            next.task_kind,
            next.target_group,
            next.target_member,
            next.delegate_model,
            next.prompt_template,
            next.is_enabled as i64,
            next.sort_order
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(next)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    let n = conn
        .execute("DELETE FROM task_route_preferences WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("未找到 task route preference".to_string());
    }
    Ok(())
}
