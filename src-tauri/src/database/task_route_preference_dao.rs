use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::database::DaoError;
use crate::domain::task_route_preference::{NewTaskRoutePreference, TaskRoutePreference};

fn normalize_non_empty(value: &str, field: &'static str) -> Result<String, DaoError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DaoError::Validation {
            field,
            message: "cannot be empty".into(),
        });
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

fn ensure_unique_task_kind(
    conn: &Connection,
    task_kind: &str,
    except_id: Option<&str>,
) -> Result<(), DaoError> {
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
            return Err(DaoError::AlreadyExists {
                entity: "task_route_preference",
                id: task_kind.to_string(),
            });
        }
    }

    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<TaskRoutePreference>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_kind, target_group, target_member, delegate_model, prompt_template, is_enabled, sort_order
         FROM task_route_preferences
         ORDER BY sort_order ASC, task_kind ASC",
    )?;
    let iter = stmt.query_map([], |row| {
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
    })?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<TaskRoutePreference>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_kind, target_group, target_member, delegate_model, prompt_template, is_enabled, sort_order
         FROM task_route_preferences
         WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(TaskRoutePreference {
            id: row.get(0)?,
            task_kind: row.get(1)?,
            target_group: row.get(2)?,
            target_member: row.get(3)?,
            delegate_model: row.get(4)?,
            prompt_template: row.get(5)?,
            is_enabled: row.get::<_, i64>(6)? != 0,
            sort_order: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn create(
    conn: &Connection,
    input: NewTaskRoutePreference,
) -> Result<TaskRoutePreference, DaoError> {
    let task_kind = normalize_non_empty(&input.task_kind, "task_kind")?;
    let target_group = normalize_non_empty(&input.target_group, "target_group")?;
    let prompt_template = normalize_optional(input.prompt_template);
    ensure_unique_task_kind(conn, &task_kind, None)?;
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM task_route_preferences",
        [],
        |row| row.get(0),
    )?;

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
    )?;

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
) -> Result<TaskRoutePreference, DaoError> {
    let current = get_by_id(conn, id)?.ok_or_else(|| DaoError::NotFound {
        entity: "task_route_preference",
        id: id.to_string(),
    })?;
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
    )?;

    Ok(next)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), DaoError> {
    let n = conn.execute("DELETE FROM task_route_preferences WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(DaoError::NotFound {
            entity: "task_route_preference",
            id: id.to_string(),
        });
    }
    Ok(())
}
