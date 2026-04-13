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
    config::app_config::load_gateway_config,
    database::{model_group_dao, model_group_member_dao},
    domain::routing::{RoutingStatus, SetActiveMemberRequest},
    gateway::{
        forwarder::{
            forward_request, forward_request_copilot_stream, forward_request_copilot_stream_openai,
            forward_request_stream_passthrough, get_provider_for_model, has_copilot_account,
        },
        protocol::{anthropic_adapter, openai_adapter},
        routes::subagent_route,
    },
    log_codes,
    runtime_events,
    service::routing_service,
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
        .route("/v1/routing/status", get(handle_routing_status))
        .route(
            "/v1/routing/groups/:alias/members",
            get(handle_group_members),
        )
        .route(
            "/v1/routing/groups/:alias/active-member",
            post(handle_set_active_member),
        )
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

    let conn = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database lock poisoned",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    let groups = model_group_dao::list(&conn).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": e,
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
                        "error": e,
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
    let conn = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database lock poisoned",
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

async fn handle_group_members(
    State(state): State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let conn = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database lock poisoned",
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
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                Json(json!({
                    "error": e.to_string(),
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
    let conn = state.db.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "database lock poisoned",
                "code": "DB_LOCK_ERROR"
            })),
        )
    })?;

    let group = routing_service::set_group_active_member_by_alias(&conn, &alias, &payload.member)
        .map_err(|e| {
            let msg = e.to_string();
            let status = if msg.contains("not found") {
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
