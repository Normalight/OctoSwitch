use std::sync::{Arc, Mutex};

use reqwest::Client;
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

use crate::config::app_config::{AppConfig, GatewayConfig};
use crate::services::{
    circuit_breaker_service::CircuitBreakerService, copilot_vendor_cache::CopilotVendorCache,
    metrics_aggregator::MetricsAggregator,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub metrics: Arc<Mutex<MetricsAggregator>>,
    pub breaker: Arc<Mutex<CircuitBreakerService>>,
    pub config: Arc<AppConfig>,
    pub restart_tx:
        Arc<Mutex<Option<mpsc::Sender<(GatewayConfig, oneshot::Sender<Result<(), String>>)>>>>,
    pub http_client: Client,
    /// Copilot `model id → vendor` 缓存（按 provider 分桶），用于 OpenAI 系模型走 `/v1/responses`
    pub copilot_vendor_cache: Arc<CopilotVendorCache>,
}
