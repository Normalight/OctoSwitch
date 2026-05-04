use crate::database::{bool_to_i64, model_binding_dao, DaoError};
use crate::domain::provider::{NewProvider, Provider};
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

const PROVIDER_COLUMNS: &str =
    "id,name,base_url,api_key_ref,timeout_ms,max_retries,is_enabled,sort_order,api_format,auth_mode FROM providers";

pub fn list(conn: &Connection) -> Result<Vec<Provider>, DaoError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PROVIDER_COLUMNS} ORDER BY sort_order ASC, updated_at DESC"
    ))?;

    let iter = stmt.query_map([], |row| {
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
    })?;

    let mut out = Vec::new();
    for p in iter {
        out.push(p?);
    }
    Ok(out)
}

/// Import config: insert with preserved id from exported JSON.
pub fn insert_with_id(conn: &Connection, p: &Provider) -> Result<(), DaoError> {
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
    )?;
    Ok(())
}

pub fn create(conn: &Connection, input: NewProvider) -> Result<Provider, DaoError> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let next_sort_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM providers",
        [],
        |row| row.get(0),
    )?;
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
    )?;

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
) -> Result<Provider, DaoError> {
    let current = get_by_id(conn, id)?.ok_or_else(|| DaoError::NotFound {
        entity: "provider",
        id: id.to_string(),
    })?;
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
    )?;

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

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Provider>, DaoError> {
    let mut stmt = conn.prepare(&format!("SELECT {PROVIDER_COLUMNS} WHERE id=?1"))?;

    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_provider(row)?))
    } else {
        Ok(None)
    }
}

/// Delete a provider and all its model bindings / group memberships.
pub fn delete(conn: &Connection, id: &str) -> Result<(), DaoError> {
    // First delete all model bindings under this provider
    let binding_ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM model_bindings WHERE provider_id=?1")?;
        let mut rows = stmt.query([id])?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? {
            ids.push(row.get(0)?);
        }
        ids
    };
    for bid in &binding_ids {
        model_binding_dao::delete(conn, bid)?;
    }
    // Delete the provider
    let n = conn.execute("DELETE FROM providers WHERE id=?1", [id])?;
    if n == 0 {
        return Err(DaoError::NotFound {
            entity: "provider",
            id: id.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::database::DaoError;
    use crate::domain::provider::NewProvider;
    use rusqlite::Connection;
    use serde_json::json;

    fn setup_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        crate::database::init_schema(&mut conn).expect("init schema");
        conn
    }

    fn create_test_provider(conn: &Connection, name: &str) -> crate::domain::provider::Provider {
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
    fn test_list_empty_returns_vec() {
        let conn = setup_db();
        let result = super::list(&conn).expect("list");
        assert!(result.is_empty());
    }

    #[test]
    fn test_create_and_get_by_id() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "test-provider");
        let got = super::get_by_id(&conn, &p.id).expect("get_by_id").expect("should exist");
        assert_eq!(got.name, "test-provider");
        assert_eq!(got.base_url, "https://api.example.com");
        assert!(got.is_enabled);
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        create_test_provider(&conn, "provider-b");
        create_test_provider(&conn, "provider-a");
        let list = super::list(&conn).expect("list");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "provider-b");
        assert_eq!(list[1].name, "provider-a");
    }

    #[test]
    fn test_update_partial_changes_name() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "old-name");
        let updated = super::update_partial(&conn, &p.id, json!({"name": "new-name"})).expect("update");
        assert_eq!(updated.name, "new-name");
        let reloaded = super::get_by_id(&conn, &p.id).expect("get").expect("exist");
        assert_eq!(reloaded.name, "new-name");
    }

    #[test]
    fn test_update_partial_changes_enabled() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "test");
        let updated = super::update_partial(&conn, &p.id, json!({"is_enabled": false})).expect("update");
        assert!(!updated.is_enabled);
        let reloaded = super::get_by_id(&conn, &p.id).expect("get").expect("exist");
        assert!(!reloaded.is_enabled);
    }

    #[test]
    fn test_delete_provider_and_cascades() {
        let conn = setup_db();
        let p = create_test_provider(&conn, "to-delete");
        super::delete(&conn, &p.id).expect("delete");
        let result = super::get_by_id(&conn, &p.id).expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let conn = setup_db();
        let result = super::get_by_id(&conn, "nonexistent-id").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_with_id_preserves_original_id() {
        let conn = setup_db();
        super::insert_with_id(&conn, &crate::domain::provider::Provider {
            id: "my-custom-id".to_string(),
            name: "custom".to_string(),
            base_url: "https://example.com".to_string(),
            api_key_ref: "key".to_string(),
            timeout_ms: 10000,
            max_retries: 1,
            is_enabled: true,
            sort_order: 99,
            api_format: None,
            auth_mode: "bearer".to_string(),
        }).expect("insert_with_id");
        let got = super::get_by_id(&conn, "my-custom-id").expect("get").expect("exist");
        assert_eq!(got.id, "my-custom-id");
        assert_eq!(got.sort_order, 99);
    }

    #[test]
    fn test_delete_nonexistent_returns_not_found() {
        let conn = setup_db();
        let result = super::delete(&conn, "nonexistent-id");
        match result {
            Err(DaoError::NotFound { entity, .. }) => assert_eq!(entity, "provider"),
            other => panic!("expected NotFound error, got {:?}", other),
        }
    }
}
