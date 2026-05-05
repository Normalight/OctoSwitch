use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::database::{model_group_dao, DaoError};

/// Associate a binding with a group (many-to-many). If the group has no active member yet,
/// set this binding as active.
pub fn add(conn: &Connection, group_id: &str, binding_id: &str) -> Result<(), DaoError> {
    conn.execute(
        "INSERT OR IGNORE INTO model_group_members (group_id, binding_id) VALUES (?1, ?2)",
        params![group_id, binding_id],
    )?;
    let g = model_group_dao::get_by_id(conn, group_id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_group",
        id: group_id.to_string(),
    })?;
    if g.active_binding_id.is_none() && is_member(conn, group_id, binding_id)? {
        model_group_dao::set_active_binding(conn, group_id, Some(binding_id))?;
    }
    Ok(())
}

pub fn remove(conn: &Connection, group_id: &str, binding_id: &str) -> Result<(), DaoError> {
    conn.execute(
        "DELETE FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
        params![group_id, binding_id],
    )?;
    sync_active_after_remove(conn, group_id, binding_id)?;
    Ok(())
}

/// After removing a member: if it was the active member, switch to another or clear.
fn sync_active_after_remove(
    conn: &Connection,
    group_id: &str,
    removed_binding_id: &str,
) -> Result<(), DaoError> {
    let g = model_group_dao::get_by_id(conn, group_id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_group",
        id: group_id.to_string(),
    })?;
    if g.active_binding_id.as_deref() != Some(removed_binding_id) {
        return Ok(());
    }
    let remaining = list_binding_ids_for_group(conn, group_id)?;
    let next = remaining.first().map(|s| s.as_str());
    model_group_dao::set_active_binding(conn, group_id, next)?;
    Ok(())
}

pub fn list_binding_ids_for_group(
    conn: &Connection,
    group_id: &str,
) -> Result<Vec<String>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT binding_id FROM model_group_members WHERE group_id = ?1 ORDER BY binding_id",
    )?;
    let rows = stmt.query_map([group_id], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn list_group_ids_for_binding(
    conn: &Connection,
    binding_id: &str,
) -> Result<Vec<String>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT group_id FROM model_group_members WHERE binding_id = ?1 ORDER BY group_id",
    )?;
    let rows = stmt.query_map([binding_id], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn group_ids_map_for_all_bindings(
    conn: &Connection,
) -> Result<HashMap<String, Vec<String>>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT binding_id, group_id FROM model_group_members ORDER BY binding_id, group_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    for r in rows {
        let (bid, gid) = r?;
        m.entry(bid).or_default().push(gid);
    }
    Ok(m)
}

pub fn is_member(conn: &Connection, group_id: &str, binding_id: &str) -> Result<bool, DaoError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM model_group_members WHERE group_id = ?1 AND binding_id = ?2",
        params![group_id, binding_id],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}

pub fn delete_all_for_binding(conn: &Connection, binding_id: &str) -> Result<(), DaoError> {
    let groups = list_group_ids_for_binding(conn, binding_id)?;
    conn.execute(
        "DELETE FROM model_group_members WHERE binding_id = ?1",
        [binding_id],
    )?;
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

/// For `GET /v1/models`: each group member maps to `(group_alias, binding_routing_name)`, sorted by group.
pub fn list_group_binding_pairs_for_catalog(
    conn: &Connection,
) -> Result<Vec<(String, String, bool, bool)>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT g.alias, b.model_name, g.is_enabled, b.is_enabled \
         FROM model_group_members m \
         JOIN model_groups g ON g.id = m.group_id \
         JOIN model_bindings b ON b.id = m.binding_id \
         ORDER BY g.sort_order ASC, g.alias ASC, b.model_name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? != 0,
            row.get::<_, i64>(3)? != 0,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn export_pairs(conn: &Connection) -> Result<Vec<(String, String)>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT group_id, binding_id FROM model_group_members ORDER BY group_id, binding_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::domain::model_binding::NewModelBinding;
    use crate::domain::model_group::NewModelGroup;
    use rusqlite::Connection;

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
        crate::database::model_binding_dao::create(conn, NewModelBinding {
            model_name: model_name.to_string(),
            provider_id: provider_id.to_string(),
            upstream_model_name: format!("up-{}", model_name),
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create binding")
    }

    fn create_test_group(conn: &Connection, alias: &str) -> crate::domain::model_group::ModelGroup {
        crate::database::model_group_dao::create(conn, NewModelGroup {
            alias: alias.to_string(),
        }).expect("create group")
    }

    #[test]
    fn test_add_member_auto_sets_active() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b.id).expect("add");
        let g = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert_eq!(g.active_binding_id, Some(b.id.clone()));
    }

    #[test]
    fn test_add_second_member_does_not_override_active() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b1 = create_test_binding(&conn, "b1", &p.id);
        let b2 = create_test_binding(&conn, "b2", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b1.id).expect("add b1");
        super::add(&conn, &g.id, &b2.id).expect("add b2");
        let g = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert_eq!(g.active_binding_id, Some(b1.id.clone()));
    }

    #[test]
    fn test_remove_member_switches_active() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b1 = create_test_binding(&conn, "b1", &p.id);
        let b2 = create_test_binding(&conn, "b2", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b1.id).expect("add b1");
        super::add(&conn, &g.id, &b2.id).expect("add b2");
        super::remove(&conn, &g.id, &b1.id).expect("remove b1");
        let g = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert_eq!(g.active_binding_id, Some(b2.id.clone()));
    }

    #[test]
    fn test_remove_last_member_clears_active() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b.id).expect("add");
        super::remove(&conn, &g.id, &b.id).expect("remove");
        let g = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert!(g.active_binding_id.is_none());
    }

    #[test]
    fn test_is_member_returns_true_false() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b.id).expect("add");
        assert!(super::is_member(&conn, &g.id, &b.id).expect("check"));
        assert!(!super::is_member(&conn, &g.id, "nonexistent").expect("check"));
    }

    #[test]
    fn test_list_binding_ids_for_group() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b1 = create_test_binding(&conn, "b1", &p.id);
        let b2 = create_test_binding(&conn, "b2", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b1.id).expect("add b1");
        super::add(&conn, &g.id, &b2.id).expect("add b2");
        let ids = super::list_binding_ids_for_group(&conn, &g.id).expect("list");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&b1.id));
        assert!(ids.contains(&b2.id));
    }

    #[test]
    fn test_list_group_ids_for_binding() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g1 = create_test_group(&conn, "g1");
        let g2 = create_test_group(&conn, "g2");
        super::add(&conn, &g1.id, &b.id).expect("add to g1");
        super::add(&conn, &g2.id, &b.id).expect("add to g2");
        let ids = super::list_group_ids_for_binding(&conn, &b.id).expect("list");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&g1.id));
        assert!(ids.contains(&g2.id));
    }

    #[test]
    fn test_group_ids_map_for_all_bindings() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b1 = create_test_binding(&conn, "b1", &p.id);
        let b2 = create_test_binding(&conn, "b2", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b1.id).expect("add b1");
        super::add(&conn, &g.id, &b2.id).expect("add b2");
        let map = super::group_ids_map_for_all_bindings(&conn).expect("map");
        assert_eq!(map.get(&b1.id).unwrap(), &vec![g.id.clone()]);
        assert_eq!(map.get(&b2.id).unwrap(), &vec![g.id.clone()]);
    }

    #[test]
    fn test_delete_all_for_binding() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g1 = create_test_group(&conn, "g1");
        let g2 = create_test_group(&conn, "g2");
        super::add(&conn, &g1.id, &b.id).expect("add to g1");
        super::add(&conn, &g2.id, &b.id).expect("add to g2");
        super::delete_all_for_binding(&conn, &b.id).expect("delete");
        assert!(!super::is_member(&conn, &g1.id, &b.id).expect("check g1"));
        assert!(!super::is_member(&conn, &g2.id, &b.id).expect("check g2"));
    }

    #[test]
    fn test_list_group_binding_pairs_for_catalog() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = create_test_binding(&conn, "b", &p.id);
        let g = create_test_group(&conn, "g");
        super::add(&conn, &g.id, &b.id).expect("add");
        let pairs = super::list_group_binding_pairs_for_catalog(&conn).expect("catalog");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "g");
        assert_eq!(pairs[0].1, "b");
        assert!(pairs[0].2);
        assert!(pairs[0].3);
    }
}
