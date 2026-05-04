use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::database::DaoError;
use crate::domain::model_group::{ModelGroup, NewModelGroup};
use crate::domain::model_slug;

pub fn list(conn: &Connection) -> Result<Vec<ModelGroup>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups ORDER BY sort_order ASC, alias",
    )?;
    let iter = stmt.query_map([], |row| {
        Ok(ModelGroup {
            id: row.get(0)?,
            alias: row.get(1)?,
            active_binding_id: row.get(2)?,
            is_enabled: row.get::<_, i64>(3)? != 0,
            sort_order: row.get(4)?,
        })
    })?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_by_alias_ci(conn: &Connection, alias: &str) -> Result<Option<ModelGroup>, DaoError> {
    let needle = alias.trim();
    if needle.is_empty() {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups WHERE lower(alias) = lower(?1)",
    )?;
    let mut rows = stmt.query([needle])?;
    if let Some(row) = rows.next()? {
        Ok(Some(ModelGroup {
            id: row.get(0)?,
            alias: row.get(1)?,
            active_binding_id: row.get(2)?,
            is_enabled: row.get::<_, i64>(3)? != 0,
            sort_order: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ModelGroup>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id,alias,active_binding_id,is_enabled,sort_order FROM model_groups WHERE id=?1",
    )?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(ModelGroup {
            id: row.get(0)?,
            alias: row.get(1)?,
            active_binding_id: row.get(2)?,
            is_enabled: row.get::<_, i64>(3)? != 0,
            sort_order: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

/// Whether an alias (case-insensitive) conflicts with an existing group.
pub fn alias_conflicts_with_model_name(
    conn: &Connection,
    alias: &str,
    exclude_group_id: Option<&str>,
) -> Result<bool, DaoError> {
    let needle = alias.trim();
    if needle.is_empty() {
        return Ok(true);
    }
    let mut stmt = conn.prepare(
        "SELECT id FROM model_groups WHERE lower(alias) = lower(?1)",
    )?;
    let mut rows = stmt.query([needle])?;
    if let Some(row) = rows.next()? {
        let gid: String = row.get(0)?;
        if exclude_group_id != Some(gid.as_str()) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn create(conn: &Connection, input: NewModelGroup) -> Result<ModelGroup, DaoError> {
    let alias = input.alias.trim().to_string();
    if alias.is_empty() {
        return Err(DaoError::Validation {
            field: "alias",
            message: "Alias cannot be empty".into(),
        });
    }
    model_slug::validate_no_slash(&alias, "alias").map_err(|msg| DaoError::Validation {
        field: "alias",
        message: msg,
    })?;
    if alias_conflicts_with_model_name(conn, &alias, None)? {
        return Err(DaoError::AlreadyExists {
            entity: "model_group",
            id: alias,
        });
    }
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM model_groups",
        [],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO model_groups (id,alias,active_binding_id,sort_order) VALUES (?1,?2,NULL,?3)",
        params![id, alias, next_sort_order],
    )?;
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
) -> Result<ModelGroup, DaoError> {
    let current = get_by_id(conn, id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_group",
        id: id.to_string(),
    })?;
    let mut next = current.clone();

    if let Some(v) = patch.get("alias").and_then(|v| v.as_str()) {
        let a = v.trim().to_string();
        if a.is_empty() {
            return Err(DaoError::Validation {
                field: "alias",
                message: "Alias cannot be empty".into(),
            });
        }
        model_slug::validate_no_slash(&a, "alias").map_err(|msg| DaoError::Validation {
            field: "alias",
            message: msg,
        })?;
        if alias_conflicts_with_model_name(conn, &a, Some(id))? {
            return Err(DaoError::AlreadyExists {
                entity: "model_group",
                id: a,
            });
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
    )?;

    Ok(next)
}

pub fn set_active_binding(
    conn: &Connection,
    group_id: &str,
    binding_id: Option<&str>,
) -> Result<ModelGroup, DaoError> {
    let mut g = get_by_id(conn, group_id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_group",
        id: group_id.to_string(),
    })?;

    match binding_id {
        None => {
            g.active_binding_id = None;
        }
        Some(bid) => {
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
                params![group_id, bid],
                |row| row.get(0),
            )?;
            if n == 0 {
                return Err(DaoError::Validation {
                    field: "binding_id",
                    message: "Binding does not belong to this group".into(),
                });
            }
            g.active_binding_id = Some(bid.to_string());
        }
    }

    conn.execute(
        "UPDATE model_groups SET active_binding_id=?2 WHERE id=?1",
        params![g.id, g.active_binding_id],
    )?;

    Ok(g)
}

/// Import config: insert with preserved id.
pub fn insert_with_id(conn: &Connection, g: &ModelGroup) -> Result<(), DaoError> {
    model_slug::validate_no_slash(g.alias.trim(), "alias").map_err(|msg| DaoError::Validation {
        field: "alias",
        message: msg,
    })?;
    conn.execute(
        "INSERT INTO model_groups (id,alias,active_binding_id,sort_order) VALUES (?1,?2,?3,?4)",
        params![g.id, g.alias, g.active_binding_id, g.sort_order],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), DaoError> {
    conn.execute("DELETE FROM model_group_members WHERE group_id = ?1", [id])?;
    let n = conn.execute("DELETE FROM model_groups WHERE id=?1", [id])?;
    if n == 0 {
        return Err(DaoError::NotFound {
            entity: "model_group",
            id: id.to_string(),
        });
    }
    Ok(())
}

/// When a binding is deleted, clear active_binding_id if any group points to it.
pub fn clear_active_if_points_to(conn: &Connection, binding_id: &str) -> Result<(), DaoError> {
    conn.execute(
        "UPDATE model_groups SET active_binding_id = NULL WHERE active_binding_id = ?1",
        [binding_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::database::DaoError;
    use crate::domain::model_group::NewModelGroup;
    use rusqlite::Connection;
    use serde_json::json;

    fn setup_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        crate::database::init_schema(&mut conn).expect("init schema");
        conn
    }

    fn create_test_provider(conn: &Connection, name: &str) -> crate::domain::provider::Provider {
        use crate::domain::provider::NewProvider;
        crate::database::provider_dao::create(conn, NewProvider {
            name: name.to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key_ref: "sk-test".to_string(),
            timeout_ms: 30000,
            max_retries: 2,
            is_enabled: true,
            api_format: Some("openai_chat".to_string()),
            auth_mode: "bearer".to_string(),
        }).expect("create provider")
    }

    fn create_test_binding(conn: &Connection, model_name: &str, provider_id: &str) -> crate::domain::model_binding::ModelBinding {
        use crate::domain::model_binding::NewModelBinding;
        crate::database::model_binding_dao::create(conn, NewModelBinding {
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            upstream_model_name: format!("up-{}", model_name),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create binding")
    }

    #[test]
    fn test_create_empty_alias_fails() {
        let conn = setup_db();
        let result = super::create(&conn, NewModelGroup {
            alias: "   ".to_string(),
        });
        match result {
            Err(DaoError::Validation { field, .. }) => assert_eq!(field, "alias"),
            other => panic!("expected Validation error, got {:?}", other),
        }
    }

    #[test]
    fn test_create_with_slash_alias_fails() {
        let conn = setup_db();
        let result = super::create(&conn, NewModelGroup {
            alias: "bad/alias".to_string(),
        });
        match result {
            Err(DaoError::Validation { field, .. }) => assert_eq!(field, "alias"),
            other => panic!("expected Validation error, got {:?}", other),
        }
    }

    #[test]
    fn test_create_and_get_by_id() {
        let conn = setup_db();
        let g = super::create(&conn, NewModelGroup {
            alias: "test-group".to_string(),
        }).expect("create");
        let got = super::get_by_id(&conn, &g.id).expect("get").expect("exist");
        assert_eq!(got.alias, "test-group");
        assert!(got.is_enabled);
    }

    #[test]
    fn test_create_and_get_by_alias_ci() {
        let conn = setup_db();
        let _ = super::create(&conn, NewModelGroup {
            alias: "CaseTest".to_string(),
        }).expect("create");
        let got = super::get_by_alias_ci(&conn, "casetest").expect("get").expect("exist");
        assert_eq!(got.alias, "CaseTest");
    }

    #[test]
    fn test_get_by_alias_ci_empty_returns_none() {
        let conn = setup_db();
        let result = super::get_by_alias_ci(&conn, "").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_create_duplicate_alias_fails() {
        let conn = setup_db();
        super::create(&conn, NewModelGroup {
            alias: "dup".to_string(),
        }).expect("first create");
        let result = super::create(&conn, NewModelGroup {
            alias: "dup".to_string(),
        });
        match result {
            Err(DaoError::AlreadyExists { entity, .. }) => assert_eq!(entity, "model_group"),
            other => panic!("expected AlreadyExists error, got {:?}", other),
        }
    }

    #[test]
    fn test_set_active_binding_success() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        // Clear the auto-set active to test explicit set
        crate::database::model_group_dao::set_active_binding(&conn, &g.id, None)
            .expect("clear active");
        let result = super::set_active_binding(&conn, &g.id, Some(&b.id)).expect("set active");
        assert_eq!(result.active_binding_id.as_deref(), Some(b.id.as_str()));
    }

    #[test]
    fn test_set_active_binding_non_member_fails() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b1 = create_test_binding(&conn, "b1", &p.id);
        let b2 = create_test_binding(&conn, "b2", &p.id);
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        // Only add b1 as member
        crate::database::model_group_member_dao::add(&conn, &g.id, &b1.id).expect("add member");
        // Clear auto-set active
        crate::database::model_group_dao::set_active_binding(&conn, &g.id, None)
            .expect("clear active");
        // Try to set b2 (non-member) as active
        let result = super::set_active_binding(&conn, &g.id, Some(&b2.id));
        match result {
            Err(DaoError::Validation { field, .. }) => assert_eq!(field, "binding_id"),
            other => panic!("expected Validation error, got {:?}", other),
        }
    }

    #[test]
    fn test_set_active_binding_none_clears_it() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        let g_before = super::get_by_id(&conn, &g.id).expect("get").expect("exist");
        assert!(g_before.active_binding_id.is_some());
        let result = super::set_active_binding(&conn, &g.id, None).expect("clear active");
        assert!(result.active_binding_id.is_none());
    }

    #[test]
    fn test_delete_group_removes_members() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        assert!(crate::database::model_group_member_dao::is_member(&conn, &g.id, &b.id)
            .expect("check"));
        super::delete(&conn, &g.id).expect("delete group");
        assert!(!crate::database::model_group_member_dao::is_member(&conn, &g.id, &b.id)
            .expect("check"));
    }

    #[test]
    fn test_clear_active_if_points_to() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        super::clear_active_if_points_to(&conn, &b.id).expect("clear");
        let g_after = super::get_by_id(&conn, &g.id).expect("get").expect("exist");
        assert!(g_after.active_binding_id.is_none());
    }

    #[test]
    fn test_update_partial_disables_group() {
        let conn = setup_db();
        let g = super::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create");
        assert!(g.is_enabled);
        let updated = super::update_partial(&conn, &g.id, json!({"is_enabled": false})).expect("update");
        assert!(!updated.is_enabled);
    }

    #[test]
    fn test_alias_conflicts_with_model_name() {
        let conn = setup_db();
        let g = super::create(&conn, NewModelGroup {
            alias: "conflict-test".to_string(),
        }).expect("create");
        // Same alias without exclude -> conflict
        assert!(super::alias_conflicts_with_model_name(&conn, "conflict-test", None)
            .expect("check"));
        // Same alias with exclude self -> no conflict
        assert!(!super::alias_conflicts_with_model_name(&conn, "conflict-test", Some(&g.id))
            .expect("check"));
        // Different alias -> no conflict
        assert!(!super::alias_conflicts_with_model_name(&conn, "no-such-alias", None)
            .expect("check"));
        // Empty alias -> conflict
        assert!(super::alias_conflicts_with_model_name(&conn, "", None).expect("check"));
    }
}
