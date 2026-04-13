use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::{json, Value};

use crate::{gateway::forwarder::forward_request, state::AppState};

/// 子代理路由：统一在 JSON 中返回 `code`（HTTP 语义数值）与 `correlation_id`，便于脚本与可观测性对齐。
pub async fn run_subagent(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let correlation_id = uuid::Uuid::new_v4().to_string();

    let model = payload
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "code": 400,
                    "correlation_id": correlation_id,
                    "error": "missing model",
                    "reason_code": "MISSING_MODEL"
                })),
            )
        })?;

    match forward_request(&state, model, payload.clone(), "/v1/chat/completions", None).await {
        Ok((upstream_status, body)) => Ok(Json(json!({
            "code": upstream_status,
            "correlation_id": correlation_id,
            "result": body
        }))),
        Err(e) => {
            let (status, Json(mut body)) = e.into_axum_response();
            let http_u16 = status.as_u16();
            if let Some(obj) = body.as_object_mut() {
                obj.insert("correlation_id".to_string(), json!(correlation_id));
                if let Some(reason) = obj.remove("code") {
                    obj.insert("reason_code".to_string(), reason);
                }
                obj.insert("code".to_string(), json!(http_u16));
            }
            Err((status, Json(body)))
        }
    }
}
