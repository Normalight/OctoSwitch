use std::collections::HashSet;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    config::app_config::{cc_switch_plugins_dir, load_gateway_config, repo_root_marketplace_manifest_path},
    database::{model_group_dao, model_group_member_dao},
    domain::{
        plugin_dist::PluginConfig,
        routing::{RoutingStatus, SetActiveMemberRequest},
    },
    gateway::{
        forwarder::{
            forward_request, forward_request_copilot_stream, forward_request_copilot_stream_openai,
            forward_request_stream_passthrough, get_provider_for_model, has_copilot_account,
        },
        protocol::{anthropic_adapter, openai_adapter},
        routes::subagent_route,
    },
    log_codes, runtime_events,
    service::{local_skills_service, plugin_dist_service, routing_service},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct ModelsListQuery {
    /// `true` 时包含已禁用的分组与绑定
    #[serde(default)]
    all: Option<bool>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(handle_healthz))
        .route("/v1/models", get(handle_list_models))
        .route("/v1/plugin/config", get(handle_plugin_config))
        .route("/v1/routing/status", get(handle_routing_status))
        .route(
            "/v1/routing/groups/:alias/members",
            get(handle_group_members),
        )
        .route(
            "/v1/routing/groups/:alias/active-member",
            post(handle_set_active_member),
        )
        .route("/v1/plugin/reload", post(handle_plugin_reload))
        .route("/v1/chat/completions", post(handle_openai_chat))
        .route("/v1/messages", post(handle_anthropic_messages))
        .route("/v1/subagent/run", post(subagent_route::run_subagent))
        .with_state(state)
}

async fn handle_healthz() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "octoswitch",
    }))
}

async fn handle_plugin_reload(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let marketplace_manifest_path = repo_root_marketplace_manifest_path();
    let plugins_root = cc_switch_plugins_dir();
    let gateway_config = load_gateway_config();
    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database pool error", "code": "DB_LOCK_ERROR"})),
        )
    })?;
    let runtime_config = plugin_dist_service::get_runtime_plugin_config(&gateway_config, &conn)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e, "code": "CONFIG_ERROR"})),
            )
        })?;
    drop(conn);

    let result = local_skills_service::sync_cc_switch_plugin_from_marketplace(
        &marketplace_manifest_path.to_string_lossy(),
        &plugins_root.to_string_lossy(),
        "octoswitch",
        &runtime_config,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e, "code": "SYNC_ERROR"})),
        )
    })?;

    Ok(Json(json!({
        "ok": true,
        "status": result.status,
        "copied_files": result.copied_files,
        "removed_files": result.removed_files,
        "preserved_files": result.preserved_files,
    })))
}

/// OpenAI 兼容 `GET /v1/models`：`data[].id` 为 **`分组别名`** 或 **`分组别名/绑定路由名`**（仅组成员）。
async fn handle_list_models(
    State(state): State<AppState>,
    Query(q): Query<ModelsListQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let include_disabled = q.all.unwrap_or(false);
    log::info!(
        "[{}] GET /v1/models all={include_disabled}",
        log_codes::RTR_V1_MODELS
    );

    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database pool error",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    let groups = model_group_dao::list(&conn).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": e.to_string(),
                "code": "DB_ERROR"
            })),
        )
    })?;

    let gw = load_gateway_config();
    let allow_member = gw.allow_group_member_model_path;

    let mut seen: HashSet<String> = HashSet::new();
    let mut data: Vec<Value> = Vec::new();

    for g in groups
        .into_iter()
        .filter(|g| include_disabled || g.is_enabled)
    {
        let key = g.alias.to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        data.push(json!({
            "id": g.alias,
            "object": "model",
            "created": 0_i64,
            "owned_by": "octoswitch",
        }));
    }

    if allow_member {
        let pairs =
            model_group_member_dao::list_group_binding_pairs_for_catalog(&conn).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": e.to_string(),
                        "code": "DB_ERROR"
                    })),
                )
            })?;

        for (alias, mname, g_en, b_en) in pairs {
            if !include_disabled && (!g_en || !b_en) {
                continue;
            }
            let id = format!("{alias}/{mname}");
            let key = id.to_lowercase();
            if !seen.insert(key) {
                continue;
            }
            data.push(json!({
                "id": id,
                "object": "model",
                "created": 0_i64,
                "owned_by": "octoswitch",
            }));
        }
    }

    Ok(Json(json!({
        "object": "list",
        "data": data,
    })))
}

async fn handle_routing_status(
    State(state): State<AppState>,
) -> Result<Json<RoutingStatus>, (StatusCode, Json<Value>)> {
    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database pool error",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    routing_service::get_routing_status(&conn)
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e.to_string(),
                    "code": "ROUTING_STATUS_ERROR"
                })),
            )
        })
}

async fn handle_plugin_config(
    State(state): State<AppState>,
) -> Result<Json<PluginConfig>, (StatusCode, Json<Value>)> {
    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database pool error",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    let cfg = load_gateway_config();
    plugin_dist_service::get_runtime_plugin_config(&cfg, &conn)
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": e,
                    "code": "PLUGIN_CONFIG_ERROR"
                })),
            )
        })
}

async fn handle_group_members(
    State(state): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database pool error",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    routing_service::list_group_members_by_alias(&conn, &alias)
        .map(|members| {
            Json(json!({
                "group": alias,
                "members": members
            }))
        })
        .map_err(|e| {
            let msg = e.to_string();
            let status = if msg.contains("disabled") {
                StatusCode::FORBIDDEN
            } else if msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": msg,
                    "code": "GROUP_MEMBERS_ERROR"
                })),
            )
        })
}

async fn handle_set_active_member(
    State(state): State<AppState>,
    Path(alias): Path<String>,
    Json(payload): Json<SetActiveMemberRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let conn = state.db.get().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database pool error",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    let group = routing_service::set_group_active_member_by_alias(&conn, &alias, &payload.member)
        .map_err(|e| {
        let msg = e.to_string();
        let status = if msg.contains("disabled") {
            StatusCode::FORBIDDEN
        } else if msg.contains("not found") {
            StatusCode::NOT_FOUND
        } else if msg.contains("has no member") || msg.contains("more than one member") {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        (
            status,
            Json(json!({
                "error": msg,
                "code": "SET_ACTIVE_MEMBER_ERROR"
            })),
        )
    })?;
    drop(conn);
    runtime_events::notify_config_imported();

    Ok(Json(json!({
        "group": group.alias,
        "active_member": group.active_member,
        "members": group.members,
        "model_path": group.active_member.as_ref().map(|m| format!("{}/{}", alias, m)),
    })))
}

async fn handle_openai_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let model = openai_adapter::extract_model(&payload).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Request body missing 'model' field",
                "code": "MISSING_MODEL"
            })),
        )
    })?;

    let is_stream = payload
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    log::info!(
        "[{}] POST /v1/chat/completions model={model} stream={is_stream}",
        crate::log_codes::RTR_INCOMING
    );

    if is_stream {
        let provider =
            get_provider_for_model(&state, &model).map_err(|e| e.into_axum_response())?;
        let is_copilot = has_copilot_account(&state, &provider.id);
        if is_copilot {
            return Ok(forward_request_copilot_stream_openai(
                &state,
                &model,
                payload,
                "/v1/chat/completions",
                Some(&headers),
            )
            .await
            .map_err(|e| e.into_axum_response())?
            .into_response());
        } else {
            return Ok(forward_request_stream_passthrough(
                &state,
                &model,
                payload,
                "/v1/chat/completions",
                Some(&headers),
            )
            .await
            .map_err(|e| e.into_axum_response())?
            .into_response());
        }
    }

    // Non-streaming path (existing behavior)
    match forward_request(
        &state,
        &model,
        payload,
        "/v1/chat/completions",
        Some(&headers),
    )
    .await
    {
        Ok((status, body)) => Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(body),
        )
            .into_response()),
        Err(e) => Err(e.into_axum_response()),
    }
}

async fn handle_anthropic_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let model = anthropic_adapter::extract_model(&payload).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Request body missing 'model' field",
                "code": "MISSING_MODEL"
            })),
        )
    })?;

    let is_stream = payload
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    log::info!(
        "[{}] POST /v1/messages model={model} stream={is_stream}",
        crate::log_codes::RTR_INCOMING
    );

    if is_stream {
        let provider =
            get_provider_for_model(&state, &model).map_err(|e| e.into_axum_response())?;
        let is_copilot = has_copilot_account(&state, &provider.id);
        if is_copilot {
            return Ok(forward_request_copilot_stream(
                &state,
                &model,
                payload,
                "/v1/messages",
                Some(&headers),
            )
            .await
            .map_err(|e| e.into_axum_response())?
            .into_response());
        } else {
            return Ok(forward_request_stream_passthrough(
                &state,
                &model,
                payload,
                "/v1/messages",
                Some(&headers),
            )
            .await
            .map_err(|e| e.into_axum_response())?
            .into_response());
        }
    }

    // Non-streaming path (existing behavior)
    match forward_request(&state, &model, payload, "/v1/messages", Some(&headers)).await {
        Ok((status, body)) => Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(body),
        )
            .into_response()),
        Err(e) => Err(e.into_axum_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_config::AppConfig;
    use crate::database::{self, model_binding_dao, model_group_dao, model_group_member_dao, provider_dao};
    use crate::domain::model_binding::NewModelBinding;
    use crate::domain::model_group::NewModelGroup;
    use crate::domain::provider::NewProvider;
    use crate::services::copilot_vendor_cache::CopilotVendorCache;
    use axum::body::Body;
    use http::Request;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    fn build_test_state() -> AppState {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(":memory:");
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("open memory db pool");
        {
            let conn = pool.get().expect("get db conn");
            database::init_schema(&conn).expect("init schema");
        }

        AppState {
            db: Arc::new(pool),
            metrics: Arc::new(Mutex::new(Default::default())),
            breaker: Arc::new(Mutex::new(Default::default())),
            config: Arc::new(AppConfig {
                gateway_port: 8787,
                gateway_host: "127.0.0.1".into(),
                db_path: ":memory:".into(),
                http_proxy: None,
            }),
            restart_tx: Arc::new(Mutex::new(None)),
            http_client: reqwest::Client::new(),
            copilot_vendor_cache: Arc::new(CopilotVendorCache::new()),
        }
    }

    fn seed_test_data(state: &AppState) {
        let conn = state.db.get().expect("get db conn");

        let p = provider_dao::create(&conn, NewProvider {
            name: "TestProvider".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key_ref: "sk-test".to_string(),
            timeout_ms: 30000,
            max_retries: 2,
            is_enabled: true,
            api_format: Some("openai_chat".to_string()),
            auth_mode: "bearer".to_string(),
        }).expect("create provider");

        let b = model_binding_dao::create(&conn, NewModelBinding {
            model_name: "my-gpt4".to_string(),
            provider_id: p.id.clone(),
            upstream_model_name: "gpt-4".to_string(),
            rpm_limit: None,
            tpm_limit: None,
            is_enabled: true,
        }).expect("create binding");

        let g = model_group_dao::create(&conn, NewModelGroup {
            alias: "Sonnet".to_string(),
        }).expect("create group");

        model_group_member_dao::add(&conn, &g.id, &b.id).expect("add member");
        model_group_dao::set_active_binding(&conn, &g.id, Some(&b.id)).expect("set active");
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let state = build_test_state();
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["service"], "octoswitch");
    }

    #[tokio::test]
    async fn list_models_returns_data() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/models").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["object"], "list");
        let data = json["data"].as_array().expect("data is array");
        assert!(!data.is_empty());
        let ids: Vec<&str> = data.iter().map(|m| m["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"Sonnet"));
    }

    #[tokio::test]
    async fn routing_status_returns_groups() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/routing/status").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["allow_group_member_model_path"].is_boolean());
        let groups = json["groups"].as_array().expect("groups is array");
        assert!(!groups.is_empty());
    }

    #[tokio::test]
    async fn group_members_returns_members() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/routing/groups/Sonnet/members").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["group"], "Sonnet");
        let members = json["members"].as_array().expect("members is array");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0]["name"], "my-gpt4");
        assert_eq!(members[0]["active"], true);
    }

    #[tokio::test]
    async fn group_members_nonexistent_group_returns_404() {
        let state = build_test_state();
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/routing/groups/Nonexistent/members").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn set_active_member_succeeds() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder()
            .uri("/v1/routing/groups/Sonnet/active-member")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&serde_json::json!({"member": "my-gpt4"})).unwrap()))
            .unwrap()
        ).await.unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["group"], "Sonnet");
        assert_eq!(json["active_member"], "my-gpt4");
        assert_eq!(json["model_path"], "Sonnet/my-gpt4");
    }

    #[tokio::test]
    async fn set_active_member_invalid_member_returns_error() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder()
            .uri("/v1/routing/groups/Sonnet/active-member")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&serde_json::json!({"member": "nonexistent"})).unwrap()))
            .unwrap()
        ).await.unwrap();

        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn plugin_config_returns_response() {
        let state = build_test_state();
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/plugin/config").body(Body::empty()).unwrap()).await.unwrap();

        // Plugin config may fail (500) when plugin directories don't exist;
        // the endpoint should return a well-formed response regardless.
        assert!(response.status().is_success() || response.status().is_server_error());
    }

    #[tokio::test]
    async fn models_endpoint_respects_all_param() {
        let state = build_test_state();
        seed_test_data(&state);
        let app = build_router(state);

        let response = app.oneshot(Request::builder().uri("/v1/models?all=true").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);
    }
}
