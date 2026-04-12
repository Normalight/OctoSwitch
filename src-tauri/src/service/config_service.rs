use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    database::{model_binding_dao, model_group_dao, model_group_member_dao, provider_dao},
    domain::{
        model_binding::ModelBinding, model_group::ModelGroup, provider::Provider,
    },
};

/// 尝试多种方式从 cc-switch 数据库读取供应商配置
fn query_cc_providers(
    cc_conn: &Connection,
) -> Result<Vec<(String, String, String, Option<String>)>, String> {
    // 方式 1：providers 表 + app_type 列
    let result = query_with_filter(cc_conn, "app_type = 'claude'");
    if !result.is_empty() {
        return Ok(result);
    }

    // 方式 2：providers 表 + 不过滤（可能没有 app_type 列）
    let result = query_with_filter(cc_conn, "1=1");
    if !result.is_empty() {
        return Ok(result);
    }

    // 方式 3：尝试 other_providers 表
    let result = query_from_table(cc_conn, "other_providers", "1=1");
    if !result.is_empty() {
        return Ok(result);
    }

    // 列出可用表名帮助调试
    let tables: Vec<String> = match cc_conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    ) {
        Ok(mut stmt) => {
            let rows: Vec<String> = stmt.query_map([], |row| row.get(0))
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|r| r.ok())
                .collect();
            rows
        }
        Err(_) => vec![],
    };

    let tables_str = tables.join(", ");
    Err(format!(
        "未找到供应商配置数据。可用表: {}",
        if tables.is_empty() { "无" } else { &tables_str }
    ))
}

fn query_with_filter(
    cc_conn: &Connection,
    filter: &str,
) -> Vec<(String, String, String, Option<String>)> {
    query_from_table(cc_conn, "providers", filter)
}

fn query_from_table(
    cc_conn: &Connection,
    table: &str,
    filter: &str,
) -> Vec<(String, String, String, Option<String>)> {
    let sql = format!("SELECT id, name, settings_config, meta FROM {table} WHERE {filter}");
    let mut stmt = match cc_conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3).ok(),
        ))
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    rows.filter_map(|row| match row {
        Ok(v) => Some(v),
        Err(_) => None,
    })
    .collect()
}

/// 默认 base_url（当 cc-switch 配置中未指定时）
const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";

/// cc-switch 导入结果报告
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub providers_imported: usize,
    pub providers_skipped: usize,
    pub models_bound: usize,
    pub models_skipped: usize,
    pub details: Vec<ImportDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportDetail {
    pub cc_name: String,
    pub status: String, // "imported" | "skipped_duplicate" | "skipped_existing"
    pub provider_id: Option<String>,
    pub models_imported: Vec<String>,
    pub models_skipped: Vec<String>,
    pub reason: Option<String>,
}

pub fn export_config(conn: &Connection) -> Result<String, String> {
    let providers = provider_dao::list(conn)?;
    let groups = model_group_dao::list(conn)?;
    let models = model_binding_dao::list(conn)?;
    let member_pairs = model_group_member_dao::export_pairs(conn)?;
    let model_group_members: Vec<_> = member_pairs
        .into_iter()
        .map(|(group_id, binding_id)| json!({ "group_id": group_id, "binding_id": binding_id }))
        .collect();
    serde_json::to_string_pretty(&json!({
        "providers": providers,
        "model_groups": groups,
        "model_bindings": models,
        "model_group_members": model_group_members
    }))
    .map_err(|e| e.to_string())
}

pub fn import_config(conn: &Connection, payload: &str) -> Result<(), String> {
    let v: serde_json::Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    let providers = v
        .get("providers")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let groups = v
        .get("model_groups")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let models = v
        .get("model_bindings")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let members = v
        .get("model_group_members")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    conn.execute("DELETE FROM model_group_members", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM model_bindings", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM model_groups", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM providers", [])
        .map_err(|e| e.to_string())?;

    for p in providers {
        let p: Provider = serde_json::from_value(p).map_err(|e| e.to_string())?;
        provider_dao::insert_with_id(conn, &p)?;
    }
    for g in groups {
        let g: ModelGroup = serde_json::from_value(g).map_err(|e| e.to_string())?;
        model_group_dao::insert_with_id(conn, &g)?;
    }
    for m in models {
        let mut m: ModelBinding = serde_json::from_value(m).map_err(|e| e.to_string())?;
        let legacy_gid = m.group_id.take();
        let from_ids = std::mem::take(&mut m.group_ids);
        model_binding_dao::insert_with_id(conn, &m)?;
        if let Some(gid) = legacy_gid.filter(|s| !s.trim().is_empty()) {
            let _ = model_group_member_dao::add(conn, &gid, &m.id);
        }
        for gid in from_ids.into_iter().filter(|s| !s.trim().is_empty()) {
            let _ = model_group_member_dao::add(conn, &gid, &m.id);
        }
    }

    for entry in members {
        let gid = entry
            .get("group_id")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let bid = entry
            .get("binding_id")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if gid.is_empty() || bid.is_empty() {
            continue;
        }
        model_group_member_dao::add(conn, gid, bid)?;
    }

    Ok(())
}

/// 从 cc-switch 数据库导入供应商配置（仅 Claude Code 部分）
///
/// 去重逻辑：
/// - 供应商按 (base_url, api_key) 判断重复，跳过已有相同配置的
/// - 模型按 model_name 去重，同一供应商下不创建重复绑定
///
/// 返回详细报告（成功数、跳过数、明细）。
fn is_self_gateway_url(url: &str, host: &str, port: u16) -> bool {
    let url_lower = url.to_lowercase();
    let url_trimmed = url_lower.trim_end_matches('/');
    let patterns = [
        format!("http://{}:{}", host, port),
        format!("http://localhost:{}", port),
        format!("http://127.0.0.1:{}", port),
        format!("http://0.0.0.0:{}", port),
        format!("http://[::1]:{}", port),
    ];
    for pattern in &patterns {
        let p = pattern.trim_end_matches('/');
        if url_trimmed == p || url_trimmed.starts_with(&format!("{}/", p)) {
            return true;
        }
    }
    false
}

pub fn import_cc_switch_providers(
    conn: &mut Connection,
    gateway_host: &str,
    gateway_port: u16,
) -> Result<ImportReport, String> {
    let db_path = get_cc_switch_db_path();
    if !db_path.exists() {
        return Err(format!("未找到 cc-switch 数据库: {}", db_path.display()));
    }

    let cc_conn = Connection::open(&db_path)
        .map_err(|e| format!("无法打开 cc-switch 数据库: {}", e))?;

    // 尝试不同的查询方式
    let settings_json = query_cc_providers(&cc_conn)
        .map_err(|e| format!("无法读取 cc-switch 数据: {e}\n数据库路径: {}", db_path.display()))?;

    if settings_json.is_empty() {
        return Err("cc-switch 数据库中没有找到 Claude Code 类型的供应商配置".to_string());
    }

    // 解析所有条目
    struct CcEntry {
        id: String,
        name: String,
        base_url: String,
        api_key_ref: String,
        model_names: Vec<String>,
        api_format: Option<String>,
        timeout_ms: i64,
    }

    let entries: Vec<CcEntry> = settings_json
        .into_iter()
        .filter_map(|(id, name, settings_config, meta_json)| {
            let settings: serde_json::Value =
                match serde_json::from_str(&settings_config).map_err(|e| format!("解析 {} 的配置失败: {}", name, e)) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };

            let (base_url, api_key_ref) = match extract_claude_config(&settings) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            // base_url 为空时使用默认值
            if api_key_ref.is_empty() {
                return None;
            }

            let base_url = if base_url.is_empty() {
                DEFAULT_ANTHROPIC_BASE_URL.to_string()
            } else {
                base_url
            };

            // 提取模型名（支持单值 / 逗号分隔 / 数组）
            let mut model_names = extract_model_names(&settings);
            if model_names.is_empty() {
                model_names.push("claude-sonnet-4-20250514".to_string());
            }

            let api_format = meta_json
                .as_ref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .and_then(|m| m.get("apiFormat").and_then(|v| v.as_str()).map(String::from))
                .map(|f| if f == "copilot" { "openai_chat".to_string() } else { f });
            let timeout_ms = extract_timeout(&settings);

            Some(Ok(CcEntry {
                id,
                name,
                base_url,
                api_key_ref,
                model_names,
                api_format,
                timeout_ms,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // 在事务中处理
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 查询已有供应商，建立 (base_url, api_key) → provider_id 映射
    let mut existing_provider_ids: HashMap<(String, String), String> = HashMap::new();
    {
        let mut stmt_existing = tx
            .prepare("SELECT id, base_url, api_key_ref FROM providers")
            .map_err(|e| e.to_string())?;
        let existing_rows = stmt_existing
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(|e| e.to_string())?;
        for r in existing_rows {
            if let Ok((pid, url, key)) = r {
                existing_provider_ids.insert((url, key), pid);
            }
        }
    }

    // 记录本轮已导入的 provider_key → provider_id
    let mut imported_provider_ids: HashMap<(String, String), String> = HashMap::new();

    let mut report = ImportReport {
        providers_imported: 0,
        providers_skipped: 0,
        models_bound: 0,
        models_skipped: 0,
        details: Vec::new(),
    };

    let mut max_sort = tx
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM providers",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    for entry in &entries {
        if is_self_gateway_url(&entry.base_url, gateway_host, gateway_port) {
            report.providers_skipped += 1;
            report.details.push(ImportDetail {
                cc_name: entry.name.clone(),
                status: "skipped_self_gateway".to_string(),
                provider_id: None,
                models_imported: vec![],
                models_skipped: entry.model_names.clone(),
                reason: Some("base_url 指向本网关，已跳过".to_string()),
            });
            continue;
        }

        let key = (entry.base_url.clone(), entry.api_key_ref.clone());
        let desired_provider_id = format!("cc_{}", entry.id);
        let desired_provider = Provider {
            id: desired_provider_id.clone(),
            name: entry.name.clone(),
            base_url: entry.base_url.clone(),
            api_key_ref: entry.api_key_ref.clone(),
            timeout_ms: entry.timeout_ms,
            max_retries: 10,
            is_enabled: true,
            sort_order: max_sort,
            api_format: entry.api_format.clone(),
            auth_mode: "bearer".to_string(),
        };

        let provider_needs_update = |existing: &Provider, desired: &Provider| {
            existing.name != desired.name
                || existing.base_url != desired.base_url
                || existing.api_key_ref != desired.api_key_ref
                || existing.timeout_ms != desired.timeout_ms
                || existing.max_retries != desired.max_retries
                || existing.is_enabled != desired.is_enabled
                || existing.api_format != desired.api_format
        };

        // 解析供应商 ID：已有 → 复用，新创建
        let provider_id = if let Some(existing) = provider_dao::get_by_id(&tx, &desired_provider_id)? {
            if provider_needs_update(&existing, &desired_provider) {
                let patch = serde_json::json!({
                    "name": desired_provider.name,
                    "base_url": desired_provider.base_url,
                    "api_key_ref": desired_provider.api_key_ref,
                    "timeout_ms": desired_provider.timeout_ms,
                    "max_retries": desired_provider.max_retries,
                    "is_enabled": desired_provider.is_enabled,
                    "api_format": desired_provider.api_format,
                });
                let _ = provider_dao::update_partial(&tx, &existing.id, patch)?;
            }
            existing_provider_ids.insert(key.clone(), existing.id.clone());
            existing.id
        } else if let Some(existing_same_key_id) = existing_provider_ids.get(&key) {
            let existing_same = provider_dao::get_by_id(&tx, existing_same_key_id)?
                .ok_or_else(|| "existing provider not found".to_string())?;
            if provider_needs_update(&existing_same, &desired_provider) {
                let patch = serde_json::json!({
                    "name": desired_provider.name,
                    "timeout_ms": desired_provider.timeout_ms,
                    "max_retries": desired_provider.max_retries,
                    "is_enabled": desired_provider.is_enabled,
                    "api_format": desired_provider.api_format,
                });
                let _ = provider_dao::update_partial(&tx, &existing_same.id, patch)?;
            }
            existing_same.id
        } else if let Some(pid) = imported_provider_ids.get(&key) {
            pid.clone()
        } else {
            // 创建新供应商
            let mut provider = desired_provider.clone();
            provider.sort_order = max_sort;

            match provider_dao::insert_with_id(&tx, &provider) {
                Ok(()) => {
                    imported_provider_ids.insert(key.clone(), provider.id.clone());
                    existing_provider_ids.insert(key.clone(), provider.id.clone());
                    max_sort += 1;
                    report.providers_imported += 1;
                }
                Err(_) => {
                    report.providers_skipped += 1;
                    report.details.push(ImportDetail {
                        cc_name: entry.name.clone(),
                        status: "skipped_existing".to_string(),
                        provider_id: None,
                        models_imported: vec![],
                        models_skipped: entry.model_names.clone(),
                        reason: Some("插入失败（可能 ID 冲突）".to_string()),
                    });
                    continue;
                }
            }
            provider.id
        };

        let mut models_imported = Vec::new();
        let mut models_skipped = Vec::new();

        for model_name in &entry.model_names {
            let existing = model_binding_dao::get_by_model_name(&tx, model_name)?;
            match existing {
                Some(binding) => {
                    if binding.provider_id == provider_id {
                        models_skipped.push(model_name.clone());
                        report.models_skipped += 1;
                    } else {
                        // 已存在同名绑定但供应商不同：更新为当前供应商，保证导入后生效。
                        let patch = serde_json::json!({
                            "provider_id": provider_id,
                            "upstream_model_name": model_name,
                            "is_enabled": true,
                        });
                        if model_binding_dao::update_partial(&tx, &binding.id, patch).is_ok() {
                            models_imported.push(model_name.clone());
                            report.models_bound += 1;
                        } else {
                            models_skipped.push(model_name.clone());
                            report.models_skipped += 1;
                        }
                    }
                }
                None => {
                    let binding = ModelBinding {
                        id: Uuid::new_v4().to_string(),
                        model_name: model_name.clone(),
                        provider_id: provider_id.clone(),
                        upstream_model_name: model_name.clone(),
                        input_price_per_1m: 0.0,
                        output_price_per_1m: 0.0,
                        rpm_limit: None,
                        tpm_limit: None,
                        is_enabled: true,
                        group_id: None,
                        group_ids: vec![],
                    };
                    if model_binding_dao::insert_with_id(&tx, &binding).is_ok() {
                        models_imported.push(model_name.clone());
                        report.models_bound += 1;
                    } else {
                        models_skipped.push(model_name.clone());
                        report.models_skipped += 1;
                    }
                }
            }
        }

        report.details.push(ImportDetail {
            cc_name: entry.name.clone(),
            status: if models_imported.is_empty() {
                "skipped_existing".to_string()
            } else {
                "imported".to_string()
            },
            provider_id: Some(provider_id),
            models_imported,
            models_skipped,
            reason: None,
        });
    }

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(report)
}

fn get_cc_switch_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch")
        .join("cc-switch.db")
}

/// 从 Claude 配置中提取 base_url 和 api_key
fn extract_claude_config(settings: &serde_json::Value) -> Result<(String, String), String> {
    let env = settings
        .get("env")
        .and_then(|v| v.as_object())
        .ok_or("缺少 env 配置")?;

    let base_url = env
        .get("ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let api_key = env
        .get("ANTHROPIC_AUTH_TOKEN")
        .or_else(|| env.get("ANTHROPIC_API_KEY"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok((base_url, api_key))
}

/// 从 settings_config 提取超时（毫秒），默认 60000
fn extract_timeout(settings: &serde_json::Value) -> i64 {
    settings
        .get("env")
        .and_then(|v| v.get("API_TIMEOUT_MS"))
        .and_then(|v| v.as_str())
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(60000)
}

fn split_models_text(value: &str) -> Vec<String> {
    value
        .split([',', ';', '\n'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn extract_model_names(settings: &serde_json::Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    if let Some(env) = settings.get("env") {
        for key in ["ANTHROPIC_MODELS", "ANTHROPIC_MODEL", "CLAUDE_MODELS", "CLAUDE_MODEL"] {
            if let Some(v) = env.get(key).and_then(|v| v.as_str()) {
                out.extend(split_models_text(v));
            }
            if let Some(arr) = env.get(key).and_then(|v| v.as_array()) {
                out.extend(arr.iter().filter_map(|x| x.as_str()).map(|s| s.trim().to_string()));
            }
        }
        // cc-switch 角色默认模型映射
        for key in [
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_REASONING_MODEL",
        ] {
            if let Some(v) = env.get(key).and_then(|v| v.as_str()) {
                out.push(v.trim().to_string());
            }
        }
    }

    for key in ["models", "model", "model_names"] {
        if let Some(v) = settings.get(key).and_then(|v| v.as_str()) {
            out.extend(split_models_text(v));
        }
        if let Some(arr) = settings.get(key).and_then(|v| v.as_array()) {
            out.extend(arr.iter().filter_map(|x| x.as_str()).map(|s| s.trim().to_string()));
        }
    }

    let mut seen = HashSet::new();
    out.retain(|m| seen.insert(m.to_lowercase()));
    out
}
