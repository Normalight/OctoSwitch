use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::database::{bool_to_i64, model_group_dao, model_group_member_dao, DaoError};
use crate::domain::model_binding::{ModelBinding, NewModelBinding};
use crate::domain::model_slug;

fn row_to_binding(row: &Row<'_>) -> rusqlite::Result<ModelBinding> {
    Ok(ModelBinding {
        id: row.get(0)?,
        model_name: row.get(1)?,
        provider_id: row.get(2)?,
        upstream_model_name: row.get(3)?,
        input_price_per_1m: row.get(4)?,
        output_price_per_1m: row.get(5)?,
        rpm_limit: row.get(6)?,
        tpm_limit: row.get(7)?,
        is_enabled: row.get::<_, i64>(8)? == 1,
        group_id: row.get(9)?,
        group_ids: vec![],
    })
}

fn attach_group_ids(
    mut bindings: Vec<ModelBinding>,
    conn: &Connection,
) -> Result<Vec<ModelBinding>, DaoError> {
    let map = model_group_member_dao::group_ids_map_for_all_bindings(conn)?;
    for b in &mut bindings {
        b.group_ids = map.get(&b.id).cloned().unwrap_or_default();
        b.group_id = None;
    }
    Ok(bindings)
}

pub fn list(conn: &Connection) -> Result<Vec<ModelBinding>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings ORDER BY model_name",
    )?;
    let iter = stmt.query_map([], row_to_binding)?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row?);
    }
    attach_group_ids(out, conn)
}

/// Import config: insert with preserved id and provider_id; `group_id` column is NULL (relationships in model_group_members).
pub fn insert_with_id(conn: &Connection, m: &ModelBinding) -> Result<(), DaoError> {
    model_slug::validate_no_slash(m.model_name.trim(), "model_name").map_err(|msg| {
        DaoError::Validation {
            field: "model_name",
            message: msg,
        }
    })?;
    conn.execute(
        "INSERT INTO model_bindings (id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            m.id,
            m.model_name,
            m.provider_id,
            m.upstream_model_name,
            m.input_price_per_1m,
            m.output_price_per_1m,
            m.rpm_limit,
            m.tpm_limit,
            bool_to_i64(m.is_enabled),
            Option::<String>::None,
        ],
    )?;
    Ok(())
}

pub fn create(conn: &Connection, input: NewModelBinding) -> Result<ModelBinding, DaoError> {
    let model_name = input.model_name.trim().to_string();
    model_slug::validate_no_slash(&model_name, "model_name").map_err(|msg| {
        DaoError::Validation {
            field: "model_name",
            message: msg,
        }
    })?;
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO model_bindings (id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            id,
            model_name.clone(),
            input.provider_id,
            input.upstream_model_name,
            input.input_price_per_1m,
            input.output_price_per_1m,
            input.rpm_limit,
            input.tpm_limit,
            bool_to_i64(input.is_enabled),
            Option::<String>::None,
        ],
    )?;

    Ok(ModelBinding {
        id: id.clone(),
        model_name,
        provider_id: input.provider_id,
        upstream_model_name: input.upstream_model_name,
        input_price_per_1m: input.input_price_per_1m,
        output_price_per_1m: input.output_price_per_1m,
        rpm_limit: input.rpm_limit,
        tpm_limit: input.tpm_limit,
        is_enabled: input.is_enabled,
        group_id: None,
        group_ids: vec![],
    })
}

pub fn update_partial(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<ModelBinding, DaoError> {
    let current = get_by_id(conn, id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_binding",
        id: id.to_string(),
    })?;
    let mut next = current.clone();

    if let Some(v) = patch.get("model_name").and_then(|v| v.as_str()) {
        let mn = v.trim().to_string();
        model_slug::validate_no_slash(&mn, "model_name").map_err(|msg| {
            DaoError::Validation {
                field: "model_name",
                message: msg,
            }
        })?;
        next.model_name = mn;
    }
    if let Some(v) = patch.get("provider_id").and_then(|v| v.as_str()) {
        next.provider_id = v.to_string();
    }
    if let Some(v) = patch.get("upstream_model_name").and_then(|v| v.as_str()) {
        next.upstream_model_name = v.to_string();
    }
    if let Some(v) = patch.get("input_price_per_1m").and_then(|v| v.as_f64()) {
        next.input_price_per_1m = v;
    }
    if let Some(v) = patch.get("output_price_per_1m").and_then(|v| v.as_f64()) {
        next.output_price_per_1m = v;
    }
    if let Some(v) = patch.get("rpm_limit") {
        next.rpm_limit = if v.is_null() { None } else { v.as_i64() };
    }
    if let Some(v) = patch.get("tpm_limit") {
        next.tpm_limit = if v.is_null() { None } else { v.as_i64() };
    }
    if let Some(v) = patch.get("is_enabled").and_then(|v| v.as_bool()) {
        next.is_enabled = v;
    }

    conn.execute(
        "UPDATE model_bindings SET model_name=?2,provider_id=?3,upstream_model_name=?4,input_price_per_1m=?5,output_price_per_1m=?6,rpm_limit=?7,tpm_limit=?8,is_enabled=?9,group_id=NULL WHERE id=?1",
        params![
            next.id,
            next.model_name,
            next.provider_id,
            next.upstream_model_name,
            next.input_price_per_1m,
            next.output_price_per_1m,
            next.rpm_limit,
            next.tpm_limit,
            bool_to_i64(next.is_enabled),
        ],
    )?;

    get_by_id(conn, id)?.ok_or_else(|| DaoError::NotFound {
        entity: "model_binding",
        id: id.to_string(),
    })
}

pub fn get_by_model_name(
    conn: &Connection,
    model_name: &str,
) -> Result<Option<ModelBinding>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE model_name=?1",
    )?;
    let mut rows = stmt.query([model_name])?;
    if let Some(row) = rows.next()? {
        let mut b = row_to_binding(&row)?;
        b.group_ids = model_group_member_dao::list_group_ids_for_binding(conn, &b.id)?;
        b.group_id = None;
        Ok(Some(b))
    } else {
        Ok(None)
    }
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ModelBinding>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE id=?1",
    )?;
    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        let mut b = row_to_binding(&row)?;
        b.group_ids = model_group_member_dao::list_group_ids_for_binding(conn, id)?;
        b.group_id = None;
        Ok(Some(b))
    } else {
        Ok(None)
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), DaoError> {
    model_group_dao::clear_active_if_points_to(conn, id)?;
    model_group_member_dao::delete_all_for_binding(conn, id)?;
    let n = conn.execute("DELETE FROM model_bindings WHERE id=?1", [id])?;
    if n == 0 {
        return Err(DaoError::NotFound {
            entity: "model_binding",
            id: id.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::database::DaoError;
    use crate::domain::model_binding::{ModelBinding, NewModelBinding};
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

    #[test]
    fn test_create_with_slash_in_model_name_fails() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let result = super::create(&conn, NewModelBinding {
            model_name: "bad/name".to_string(),
            provider_id: p.id,
            upstream_model_name: "gpt-4".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        });
        match result {
            Err(DaoError::Validation { field, .. }) => assert_eq!(field, "model_name"),
            other => panic!("expected Validation error, got {:?}", other),
        }
    }

    #[test]
    fn test_create_and_get_by_id() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "my-model".to_string(),
            provider_id: p.id.clone(),
            upstream_model_name: "up-model".to_string(),
            input_price_per_1m: 1.0,
            output_price_per_1m: 2.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let got = super::get_by_id(&conn, &b.id).expect("get").expect("exist");
        assert_eq!(got.id, b.id);
        assert_eq!(got.model_name, "my-model");
        assert_eq!(got.provider_id, p.id);
    }

    #[test]
    fn test_create_and_get_by_model_name() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "unique-model".to_string(),
            provider_id: p.id,
            upstream_model_name: "up-model".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let got = super::get_by_model_name(&conn, "unique-model").expect("get").expect("exist");
        assert_eq!(got.id, b.id);
    }

    #[test]
    fn test_list_attaches_group_ids() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "m1".to_string(),
            provider_id: p.id,
            upstream_model_name: "up".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let g = crate::database::model_group_dao::create(&conn, NewModelGroup {
            alias: "g1".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        let list = super::list(&conn).expect("list");
        let found = list.iter().find(|x| x.id == b.id).expect("find binding");
        assert_eq!(found.group_ids, vec![g.id.clone()]);
    }

    #[test]
    fn test_update_partial_changes_provider_id() {
        let conn = setup_db();
        let p1 = create_test_provider(&conn, "p1");
        let p2 = create_test_provider(&conn, "p2");
        let b = super::create(&conn, NewModelBinding {
            model_name: "m".to_string(),
            provider_id: p1.id.clone(),
            upstream_model_name: "up".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let updated = super::update_partial(&conn, &b.id, json!({"provider_id": p2.id})).expect("update");
        assert_eq!(updated.provider_id, p2.id);
    }

    #[test]
    fn test_update_partial_changes_prices() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "m".to_string(),
            provider_id: p.id,
            upstream_model_name: "up".to_string(),
            input_price_per_1m: 1.0,
            output_price_per_1m: 2.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let updated = super::update_partial(
            &conn, &b.id,
            json!({"input_price_per_1m": 5.0, "output_price_per_1m": 10.0}),
        ).expect("update");
        assert_eq!(updated.input_price_per_1m, 5.0);
        assert_eq!(updated.output_price_per_1m, 10.0);
    }

    #[test]
    fn test_update_partial_null_rpm_limit() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "m".to_string(),
            provider_id: p.id,
            upstream_model_name: "up".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: Some(100),
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        assert_eq!(b.rpm_limit, Some(100));
        let updated = super::update_partial(&conn, &b.id, json!({"rpm_limit": null})).expect("update");
        assert!(updated.rpm_limit.is_none());
    }

    #[test]
    fn test_delete_binding_clears_active_and_members() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        let b = super::create(&conn, NewModelBinding {
            model_name: "m".to_string(),
            provider_id: p.id,
            upstream_model_name: "up".to_string(),
            input_price_per_1m: 0.0,
            output_price_per_1m: 0.0,
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create");
        let g = crate::database::model_group_dao::create(&conn, NewModelGroup {
            alias: "g".to_string(),
        }).expect("create group");
        crate::database::model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        let g_before = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert_eq!(g_before.active_binding_id, Some(b.id.clone()));
        super::delete(&conn, &b.id).expect("delete");
        let g_after = crate::database::model_group_dao::get_by_id(&conn, &g.id)
            .expect("get").expect("exist");
        assert!(g_after.active_binding_id.is_none());
        assert!(super::get_by_id(&conn, &b.id).expect("get").is_none());
    }

    #[test]
    fn test_insert_with_id_preserves_fields() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "p");
        super::insert_with_id(&conn, &ModelBinding {
            id: "custom-bid".to_string(),
            model_name: "custom-model".to_string(),
            provider_id: p.id.clone(),
            upstream_model_name: "upstream".to_string(),
            input_price_per_1m: 3.5,
            output_price_per_1m: 7.0,
            rpm_limit: Some(200),
            tpm_limit: Some(500),
            is_enabled: true,
            group_id: None,
            group_ids: vec![],
        }).expect("insert_with_id");
        let got = super::get_by_id(&conn, "custom-bid").expect("get").expect("exist");
        assert_eq!(got.id, "custom-bid");
        assert_eq!(got.model_name, "custom-model");
        assert_eq!(got.provider_id, p.id);
        assert_eq!(got.input_price_per_1m, 3.5);
        assert_eq!(got.output_price_per_1m, 7.0);
        assert_eq!(got.rpm_limit, Some(200));
        assert_eq!(got.tpm_limit, Some(500));
    }
}
