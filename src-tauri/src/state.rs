use std::sync::{Arc, Mutex};

use reqwest::Client;
use tokio::sync::{mpsc, oneshot};

use crate::config::app_config::{AppConfig, GatewayConfig};
use crate::services::{
    circuit_breaker_service::CircuitBreakerService, copilot_vendor_cache::CopilotVendorCache,
    metrics_aggregator::MetricsAggregator,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,
    pub metrics: Arc<Mutex<MetricsAggregator>>,
    pub breaker: Arc<Mutex<CircuitBreakerService>>,
    /// Static app paths/proxy etc.; retained for future commands that need `AppConfig` without reload.
    #[allow(dead_code)]
    pub config: Arc<AppConfig>,
    pub restart_tx:
        Arc<Mutex<Option<mpsc::Sender<(GatewayConfig, oneshot::Sender<Result<(), String>>)>>>>,
    pub http_client: Client,
    /// Copilot `model id → vendor` 缓存（按 provider 分桶），用于 OpenAI 系模型走 `/v1/responses`
    pub copilot_vendor_cache: Arc<CopilotVendorCache>,
}
