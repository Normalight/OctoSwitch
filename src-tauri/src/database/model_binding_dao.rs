use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::database::{model_group_dao, model_group_member_dao};
use crate::domain::model_binding::{ModelBinding, NewModelBinding};
use crate::domain::model_slug;

use super::bool_to_i64;

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
) -> Result<Vec<ModelBinding>, String> {
    let map = model_group_member_dao::group_ids_map_for_all_bindings(conn)?;
    for b in &mut bindings {
        b.group_ids = map.get(&b.id).cloned().unwrap_or_default();
        b.group_id = None;
    }
    Ok(bindings)
}

pub fn list(conn: &Connection) -> Result<Vec<ModelBinding>, String> {
    let mut stmt = conn
        .prepare("SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings ORDER BY model_name")
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], row_to_binding)
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row.map_err(|e| e.to_string())?);
    }
    attach_group_ids(out, conn)
}

/// 导入配置时保留 id 与 provider_id；`group_id` 列写入 NULL（关系见 model_group_members）
pub fn insert_with_id(conn: &Connection, m: &ModelBinding) -> Result<(), String> {
    model_slug::validate_no_slash(m.model_name.trim(), "模型路由名")?;
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
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create(conn: &Connection, input: NewModelBinding) -> Result<ModelBinding, String> {
    let model_name = input.model_name.trim().to_string();
    model_slug::validate_no_slash(&model_name, "模型路由名")?;
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
    )
    .map_err(|e| e.to_string())?;

    let binding = ModelBinding {
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
    };

    Ok(binding)
}

pub fn update_partial(
    conn: &Connection,
    id: &str,
    patch: serde_json::Value,
) -> Result<ModelBinding, String> {
    let current = get_by_id(conn, id)?.ok_or_else(|| "model binding not found".to_string())?;
    let mut next = current.clone();

    if let Some(v) = patch.get("model_name").and_then(|v| v.as_str()) {
        let mn = v.trim().to_string();
        model_slug::validate_no_slash(&mn, "模型路由名")?;
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
    )
    .map_err(|e| e.to_string())?;

    get_by_id(conn, id)?.ok_or_else(|| "update lost row".to_string())
}

pub fn get_by_model_name(
    conn: &Connection,
    model_name: &str,
) -> Result<Option<ModelBinding>, String> {
    let mut stmt = conn
        .prepare("SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE model_name=?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([model_name]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut b = row_to_binding(&row).map_err(|e| e.to_string())?;
        b.group_ids = model_group_member_dao::list_group_ids_for_binding(conn, &b.id)?;
        b.group_id = None;
        Ok(Some(b))
    } else {
        Ok(None)
    }
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<ModelBinding>, String> {
    let mut stmt = conn
        .prepare("SELECT id,model_name,provider_id,upstream_model_name,input_price_per_1m,output_price_per_1m,rpm_limit,tpm_limit,is_enabled,group_id FROM model_bindings WHERE id=?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let mut b = row_to_binding(&row).map_err(|e| e.to_string())?;
        b.group_ids = model_group_member_dao::list_group_ids_for_binding(conn, id)?;
        b.group_id = None;
        Ok(Some(b))
    } else {
        Ok(None)
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    model_group_dao::clear_active_if_points_to(conn, id)?;
    model_group_member_dao::delete_all_for_binding(conn, id)?;
    let n = conn
        .execute("DELETE FROM model_bindings WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("未找到该模型绑定".to_string());
    }
    Ok(())
}
