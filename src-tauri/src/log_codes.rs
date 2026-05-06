//! Centralized log code constants for grep-ability.
//! Usage: `log::warn!("[{CB_OPEN}] provider '{provider_id}' circuit breaker OPEN");`

// ── Application lifecycle ──
pub const APP_START: &str = "APP-001";

// ── Gateway lifecycle ──
pub const GW_START: &str = "GW-001";
pub const GW_BIND: &str = "GW-002";
pub const GW_ERROR: &str = "GW-003";
pub const GW_RESTART: &str = "GW-004";
#[allow(dead_code)]
pub const GW_STOP: &str = "GW-005";

// ── Router / access logging ──
pub const RTR_INCOMING: &str = "RTR-001";
/// `GET /v1/models`（OpenAI 列表形态，供 cc-switch 等自动发现可用 `model`）
pub const RTR_V1_MODELS: &str = "RTR-002";

// ── Forwarding ──
pub const FWD_START: &str = "FWD-001";
pub const FWD_DONE: &str = "FWD-002";
#[allow(dead_code)]
pub const FWD_ERROR: &str = "FWD-003";
pub const FWD_RETRY: &str = "FWD-004";
pub const FWD_RETRY_EXH: &str = "FWD-005";

// ── Streaming ──
pub const STRM_START: &str = "STRM-001";
pub const STRM_DONE: &str = "STRM-002";
pub const STRM_ERROR: &str = "STRM-003";
pub const STRM_EOF: &str = "STRM-004";
pub const STRM_DISCONNECT: &str = "STRM-005";

// ── Circuit breaker ──
pub const CB_OPEN: &str = "CB-001";
pub const CB_CLOSED: &str = "CB-002";
#[allow(dead_code)]
pub const CB_REJECTED: &str = "CB-003";

// ── Metrics ──
#[allow(dead_code)]
pub const MET_RECORD: &str = "MET-001";
pub const MET_HYDRATE: &str = "MET-002";
pub const MET_PERSIST: &str = "MET-005";
#[allow(dead_code)]
pub const MET_ERROR: &str = "MET-003";
pub const MET_LOCK_SKIP: &str = "MET-004";
pub const DB_LOCK_SKIP: &str = "DB-004";

// ── Copilot auth ──
pub const COP_AUTH_START: &str = "COP-001";
pub const COP_AUTH_REFRESH: &str = "COP-002";
#[allow(dead_code)]
pub const COP_AUTH_OK: &str = "COP-003";
pub const COP_AUTH_FAIL: &str = "COP-004";
pub const COP_TOKEN_PERSIST: &str = "COP-005";
pub const COP_DISCOVER: &str = "COP-006";
/// Copilot `GET …/models`：成功路径、按路径回退、过滤内部路由 id
pub const COP_MODELS: &str = "COP-007";
/// Copilot 模型 `vendor` 缓存与 `/v1/responses` 路由选择
pub const COP_VENDOR: &str = "COP-008";

/// 上游模型列表（OpenAI 兼容 `/v1/models` 或 UI `fetch_upstream_models`）
pub const MDL_FETCH: &str = "MDL-001";

// ── Database ──
pub const DB_INIT: &str = "DB-001";
pub const DB_MIGRATE: &str = "DB-002";
