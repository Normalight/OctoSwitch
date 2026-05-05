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
        rpm_limit: row.get(4)?,
        tpm_limit: row.get(5)?,
        is_enabled: row.get::<_, i64>(6)? == 1,
        group_id: row.get(7)?,
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
        "SELECT id,model_name,provider_id,upstream_model_name,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings ORDER BY model_name",
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
        "INSERT INTO model_bindings (id,model_name,provider_id,upstream_model_name,rpm_limit,tpm_limit,is_enabled,group_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            m.id,
            m.model_name,
            m.provider_id,
            m.upstream_model_name,
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
        "INSERT INTO model_bindings (id,model_name,provider_id,upstream_model_name,rpm_limit,tpm_limit,is_enabled,group_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id,
            model_name.clone(),
            input.provider_id,
            input.upstream_model_name,
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
        "UPDATE model_bindings SET model_name=?2,provider_id=?3,upstream_model_name=?4,rpm_limit=?5,tpm_limit=?6,is_enabled=?7,group_id=NULL WHERE id=?1",
        params![
            next.id,
            next.model_name,
            next.provider_id,
            next.upstream_model_name,
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
        "SELECT id,model_name,provider_id,upstream_model_name,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE model_name=?1",
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
        "SELECT id,model_name,provider_id,upstream_model_name,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE id=?1",
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
    use rstest::*;

    use crate::database::DaoError;
    use crate::domain::model_binding::{ModelBinding, NewModelBinding};
    use crate::domain::model_group::NewModelGroup;
    use crate::test_utils;
    use rusqlite::Connection;
    use serde_json::json;

    #[fixture]
    fn db() -> Connection {
        test_utils::test_db()
    }

    #[rstest]
    #[case("bad/name")]
    #[case("also/bad")]
    #[case("a/b/c")]
    fn test_create_with_slash_in_model_name_fails(db: Connection, #[case] model_name: String) {
        let p = test_utils::test_provider_named(&db, "p");
        let result = super::create(
            &db,
            NewModelBinding {
                model_name,
                provider_id: p.id,
                upstream_model_name: "gpt-4".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        );
        match result {
            Err(DaoError::Validation { field, .. }) => assert_eq!(field, "model_name"),
            other => panic!("expected Validation error, got {:?}", other),
        }
    }

    #[rstest]
    fn test_create_and_get_by_id(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "my-model".to_string(),
                provider_id: p.id.clone(),
                upstream_model_name: "up-model".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        let got = super::get_by_id(&db, &b.id).expect("get").expect("exist");
        assert_eq!(got.id, b.id);
        assert_eq!(got.model_name, "my-model");
        assert_eq!(got.provider_id, p.id);
    }

    #[rstest]
    fn test_create_and_get_by_model_name(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "unique-model".to_string(),
                provider_id: p.id,
                upstream_model_name: "up-model".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        let got = super::get_by_model_name(&db, "unique-model")
            .expect("get")
            .expect("exist");
        assert_eq!(got.id, b.id);
    }

    #[rstest]
    fn test_list_attaches_group_ids(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "m1".to_string(),
                provider_id: p.id,
                upstream_model_name: "up".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        let g = crate::database::model_group_dao::create(
            &db,
            NewModelGroup {
                alias: "g1".to_string(),
            },
        )
        .expect("create group");
        crate::database::model_group_member_dao::add(&db, &g.id, &b.id).expect("add member");
        let list = super::list(&db).expect("list");
        let found = list.iter().find(|x| x.id == b.id).expect("find binding");
        assert_eq!(found.group_ids, vec![g.id.clone()]);
    }

    #[rstest]
    fn test_update_partial_changes_provider_id(db: Connection) {
        let p1 = test_utils::test_provider_named(&db, "p1");
        let p2 = test_utils::test_provider_named(&db, "p2");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "m".to_string(),
                provider_id: p1.id.clone(),
                upstream_model_name: "up".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        let updated =
            super::update_partial(&db, &b.id, json!({"provider_id": p2.id})).expect("update");
        assert_eq!(updated.provider_id, p2.id);
    }

    #[rstest]
    fn test_update_partial_null_rpm_limit(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "m".to_string(),
                provider_id: p.id,
                upstream_model_name: "up".to_string(),
                rpm_limit: Some(100),
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        assert_eq!(b.rpm_limit, Some(100));
        let updated =
            super::update_partial(&db, &b.id, json!({"rpm_limit": null})).expect("update");
        assert!(updated.rpm_limit.is_none());
    }

    #[rstest]
    fn test_delete_binding_clears_active_and_members(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        let b = super::create(
            &db,
            NewModelBinding {
                model_name: "m".to_string(),
                provider_id: p.id,
                upstream_model_name: "up".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create");
        let g = crate::database::model_group_dao::create(
            &db,
            NewModelGroup {
                alias: "g".to_string(),
            },
        )
        .expect("create group");
        crate::database::model_group_member_dao::add(&db, &g.id, &b.id).expect("add member");
        let g_before = crate::database::model_group_dao::get_by_id(&db, &g.id)
            .expect("get")
            .expect("exist");
        assert_eq!(g_before.active_binding_id, Some(b.id.clone()));
        super::delete(&db, &b.id).expect("delete");
        let g_after = crate::database::model_group_dao::get_by_id(&db, &g.id)
            .expect("get")
            .expect("exist");
        assert!(g_after.active_binding_id.is_none());
        assert!(super::get_by_id(&db, &b.id).expect("get").is_none());
    }

    #[rstest]
    fn test_insert_with_id_preserves_fields(db: Connection) {
        let p = test_utils::test_provider_named(&db, "p");
        super::insert_with_id(
            &db,
            &ModelBinding {
                id: "custom-bid".to_string(),
                model_name: "custom-model".to_string(),
                provider_id: p.id.clone(),
                upstream_model_name: "upstream".to_string(),
                rpm_limit: Some(200),
                tpm_limit: Some(500),
                is_enabled: true,
                group_id: None,
                group_ids: vec![],
            },
        )
        .expect("insert_with_id");
        let got = super::get_by_id(&db, "custom-bid")
            .expect("get")
            .expect("exist");
        assert_eq!(got.id, "custom-bid");
        assert_eq!(got.model_name, "custom-model");
        assert_eq!(got.provider_id, p.id);
        assert_eq!(got.rpm_limit, Some(200));
        assert_eq!(got.tpm_limit, Some(500));
    }
}
