use chrono::{DateTime, Duration, Utc};
use tauri::State;

use crate::{
    services::{
        metrics_aggregator::{MetricKpi, MetricPoint},
        metrics_collector::{
            aggregate_usage_totals, list_request_logs_in_range,
            load_metric_bucket_aggregates_in_range,
        },
    },
    state::AppState,
};

/// 自定义区间最大跨度（与「约一个月」一致，便于本地库查询与 UI 约束）
const MAX_CUSTOM_RANGE_SECS: i64 = 31 * 24 * 3600;

fn resolve_usage_range(
    window: &str,
    custom_start: Option<&str>,
    custom_end: Option<&str>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let now = Utc::now();
    match window {
        "5m" => Ok((now - Duration::minutes(5), now)),
        "1h" => Ok((now - Duration::hours(1), now)),
        "24h" => Ok((now - Duration::hours(24), now)),
        "30d" => Ok((now - Duration::days(30), now)),
        "custom" => {
            let cs = custom_start.ok_or_else(|| "custom_start is required".to_string())?;
            let ce = custom_end.ok_or_else(|| "custom_end is required".to_string())?;
            let start = DateTime::parse_from_rfc3339(cs)
                .map_err(|e| format!("custom_start: {e}"))?
                .with_timezone(&Utc);
            let end = DateTime::parse_from_rfc3339(ce)
                .map_err(|e| format!("custom_end: {e}"))?
                .with_timezone(&Utc);
            if start >= end {
                return Err("custom range: start must be before end".into());
            }
            let leeway = Duration::minutes(2);
            if end > now + leeway {
                return Err("custom range: end cannot be in the future".into());
            }
            let span_secs = (end - start).num_seconds();
            if span_secs > MAX_CUSTOM_RANGE_SECS {
                return Err(format!(
                    "custom range: span cannot exceed {} days",
                    MAX_CUSTOM_RANGE_SECS / 86400
                ));
            }
            Ok((start, end))
        }
        _ => Err(format!("unknown usage window: {window}")),
    }
}

fn range_bounds_rfc3339(start: DateTime<Utc>, end: DateTime<Utc>) -> (String, String) {
    (
        start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )
}

fn bucket_secs_for_span(span_secs: i64) -> i64 {
    if span_secs <= 5 * 60 {
        30
    } else if span_secs <= 60 * 60 {
        300
    } else if span_secs <= 48 * 3600 {
        3600
    } else if span_secs <= 14 * 24 * 3600 {
        6 * 3600
    } else {
        86400
    }
}

fn bucket_secs_for_window(
    window: &str,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> i64 {
    match window {
        "5m" => 30,
        "1h" => 300,
        "24h" => 3600,
        "30d" => 86400,
        "custom" => bucket_secs_for_span((range_end - range_start).num_seconds()),
        _ => 3600,
    }
}

#[tauri::command]
pub fn get_metrics_kpi(
    state: State<AppState>,
    window: String,
    custom_start: Option<String>,
    custom_end: Option<String>,
) -> Result<MetricKpi, String> {
    let (start, end) =
        resolve_usage_range(&window, custom_start.as_deref(), custom_end.as_deref())?;
    let (s, e) = range_bounds_rfc3339(start, end);

    let conn = state
        .db
        .lock()
        .map_err(|_| "db lock poisoned".to_string())?;
    let aggregates = load_metric_bucket_aggregates_in_range(
        &conn,
        start,
        end,
        (end - start).num_seconds().max(1),
    )?;
    let (total_req, total_err) = aggregates.iter().fold((0i64, 0i64), |(req, err), b| {
        (req + b.request_count, err + b.error_count)
    });
    let (ti, to, tcr) = aggregate_usage_totals(&conn, &s, &e)?;

    let cnt = total_req as f64;
    let error_rate = if cnt > 0.0 {
        total_err as f64 / cnt
    } else {
        0.0
    };

    Ok(MetricKpi {
        error_rate,
        total_input_tokens: ti,
        total_output_tokens: to,
        total_cache_read_tokens: tcr,
        total_consumed_tokens: ti + tcr + to,
    })
}

#[tauri::command]
pub fn get_metrics_series(
    state: State<AppState>,
    window: String,
    custom_start: Option<String>,
    custom_end: Option<String>,
) -> Result<Vec<MetricPoint>, String> {
    let (start, end) =
        resolve_usage_range(&window, custom_start.as_deref(), custom_end.as_deref())?;
    let bucket = bucket_secs_for_window(&window, start, end);
    let conn = state
        .db
        .lock()
        .map_err(|_| "db lock poisoned".to_string())?;
    let buckets = load_metric_bucket_aggregates_in_range(&conn, start, end, bucket)?;

    if buckets.iter().all(|b| b.request_count == 0) {
        return Ok(vec![]);
    }

    let mut out = Vec::with_capacity(buckets.len());
    for b in buckets {
        let b0 = DateTime::<Utc>::from_timestamp(b.bucket_epoch, 0)
            .ok_or_else(|| "invalid bucket timestamp".to_string())?;
        out.push(MetricPoint {
            bucket_time: b0.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            group_name: b.group_name,
            provider_name: b.provider_name,
            model_name: b.model_name,
            input_tokens: b.input_tokens,
            output_tokens: b.output_tokens,
            cache_read_tokens: b.cache_read_input_tokens,
            consumed_tokens: b.input_tokens + b.cache_read_input_tokens + b.output_tokens,
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn list_request_logs(
    state: State<AppState>,
    window: String,
    custom_start: Option<String>,
    custom_end: Option<String>,
) -> Result<Vec<crate::services::metrics_collector::RequestLog>, String> {
    let (start, end) =
        resolve_usage_range(&window, custom_start.as_deref(), custom_end.as_deref())?;
    let (s, e) = range_bounds_rfc3339(start, end);
    let conn = state
        .db
        .lock()
        .map_err(|_| "db lock poisoned".to_string())?;
    list_request_logs_in_range(&conn, Some(&s), Some(&e))
}
