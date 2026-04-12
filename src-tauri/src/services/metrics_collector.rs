use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, Connection};
use serde::Serialize;
use uuid::Uuid;

use super::metrics_aggregator::{MetricSample, MetricsAggregator};

#[derive(Debug, Clone, Serialize)]
pub struct RequestLog {
    pub id: String,
    pub group_name: String,
    pub model_name: String,
    pub provider_name: String,
    pub latency_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub status_code: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RequestMetricInput {
    pub model_name: String,
    pub group_name: Option<String>,
    pub provider_id: String,
    pub status_code: i64,
    pub latency_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cost: f64,
}

pub fn record_request_metric(
    conn: &Connection,
    aggregator: &mut MetricsAggregator,
    input: RequestMetricInput,
) -> Result<(), String> {
    let now = Utc::now();
    conn.execute(
        "INSERT INTO request_logs (id,group_name,model_name,provider_id,status_code,latency_ms,input_tokens,output_tokens,cache_creation_input_tokens,cache_read_input_tokens,total_cost,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            Uuid::new_v4().to_string(),
            input.group_name.unwrap_or_default(),
            input.model_name,
            input.provider_id,
            input.status_code,
            input.latency_ms,
            input.input_tokens,
            input.output_tokens,
            input.cache_creation_input_tokens,
            input.cache_read_input_tokens,
            input.cost,
            now.to_rfc3339()
        ],
    )
    .map_err(|e| e.to_string())?;

    aggregator.push(MetricSample {
        at: now,
        latency_ms: input.latency_ms,
        input_tokens: input.input_tokens,
        output_tokens: input.output_tokens,
        cache_creation_input_tokens: input.cache_creation_input_tokens,
        cache_read_input_tokens: input.cache_read_input_tokens,
        cost: input.cost,
        is_error: input.status_code >= 400,
    });
    Ok(())
}

pub fn aggregate_usage_totals(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<(i64, i64, i64, i64, f64), String> {
    let mut stmt = conn
        .prepare(
            "SELECT \
             COALESCE(SUM(input_tokens), 0), \
             COALESCE(SUM(output_tokens), 0), \
             COALESCE(SUM(cache_creation_input_tokens), 0), \
             COALESCE(SUM(cache_read_input_tokens), 0), \
             COALESCE(SUM(total_cost), 0.0) \
             FROM request_logs WHERE created_at >= ?1 AND created_at <= ?2",
        )
        .map_err(|e| e.to_string())?;
    stmt
        .query_row([start, end], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn load_metric_samples_in_range(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<Vec<MetricSample>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT created_at, latency_ms, input_tokens, output_tokens, total_cost, status_code, \
             COALESCE(cache_creation_input_tokens, 0), COALESCE(cache_read_input_tokens, 0) \
             FROM request_logs WHERE created_at >= ?1 AND created_at <= ?2 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([start, end]).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let created: String = row.get(0).map_err(|e| e.to_string())?;
        let at: DateTime<Utc> = DateTime::parse_from_rfc3339(&created)
            .map_err(|e| e.to_string())?
            .with_timezone(&Utc);
        let latency_ms: i64 = row.get(1).map_err(|e| e.to_string())?;
        let input_tokens: i64 = row.get(2).map_err(|e| e.to_string())?;
        let output_tokens: i64 = row.get(3).map_err(|e| e.to_string())?;
        let cost: f64 = row.get(4).map_err(|e| e.to_string())?;
        let status_code: i64 = row.get(5).map_err(|e| e.to_string())?;
        let cache_creation_input_tokens: i64 = row.get(6).map_err(|e| e.to_string())?;
        let cache_read_input_tokens: i64 = row.get(7).map_err(|e| e.to_string())?;
        out.push(MetricSample {
            at,
            latency_ms,
            input_tokens,
            output_tokens,
            cache_creation_input_tokens,
            cache_read_input_tokens,
            cost,
            is_error: status_code >= 400,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct MetricBucketAggregate {
    pub bucket_epoch: i64,
    pub request_count: i64,
    pub error_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cost: f64,
}

pub fn load_metric_bucket_aggregates_in_range(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    bucket_secs: i64,
) -> Result<Vec<MetricBucketAggregate>, String> {
    if bucket_secs <= 0 || end <= start {
        return Ok(vec![]);
    }

    let start_epoch = start.timestamp();
    let end_epoch = end.timestamp();
    let bucket_count = ((end_epoch - start_epoch + bucket_secs - 1) / bucket_secs) as usize;
    if bucket_count == 0 {
        return Ok(vec![]);
    }

    let mut out = Vec::with_capacity(bucket_count);
    for i in 0..bucket_count {
        out.push(MetricBucketAggregate {
            bucket_epoch: start_epoch + (i as i64) * bucket_secs,
            request_count: 0,
            error_count: 0,
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost: 0.0,
        });
    }

    let start_s = start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let end_s = end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut stmt = conn
        .prepare(
            "SELECT
                (?1 + ((CAST(strftime('%s', created_at) AS INTEGER) - ?1) / ?2) * ?2) AS bucket_epoch,
                COUNT(*) AS request_count,
                SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) AS error_count,
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(output_tokens), 0) AS output_tokens,
                COALESCE(SUM(cache_creation_input_tokens), 0) AS cache_creation_input_tokens,
                COALESCE(SUM(cache_read_input_tokens), 0) AS cache_read_input_tokens,
                COALESCE(SUM(total_cost), 0.0) AS total_cost
             FROM request_logs
             WHERE created_at >= ?3 AND created_at <= ?4
             GROUP BY bucket_epoch
             ORDER BY bucket_epoch ASC",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query(params![start_epoch, bucket_secs, start_s, end_s])
        .map_err(|e| e.to_string())?;

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let bucket_epoch: i64 = row.get(0).map_err(|e| e.to_string())?;
        if bucket_epoch < start_epoch || bucket_epoch >= end_epoch {
            continue;
        }
        let idx = ((bucket_epoch - start_epoch) / bucket_secs) as usize;
        if idx >= out.len() {
            continue;
        }
        out[idx].request_count = row.get(1).map_err(|e| e.to_string())?;
        out[idx].error_count = row.get(2).map_err(|e| e.to_string())?;
        out[idx].input_tokens = row.get(3).map_err(|e| e.to_string())?;
        out[idx].output_tokens = row.get(4).map_err(|e| e.to_string())?;
        out[idx].cache_creation_input_tokens = row.get(5).map_err(|e| e.to_string())?;
        out[idx].cache_read_input_tokens = row.get(6).map_err(|e| e.to_string())?;
        out[idx].cost = row.get(7).map_err(|e| e.to_string())?;
    }

    Ok(out)
}

/// `start` / `end` 为 RFC3339（与落库 `created_at` 可比）；任一端缺省则不截断该侧。
pub fn list_request_logs_in_range(
    conn: &Connection,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<Vec<RequestLog>, String> {
    let mut sql = String::from(
        "SELECT r.id, COALESCE(r.group_name, ''), r.model_name, COALESCE(p.name, ''), \
         r.latency_ms, r.input_tokens, r.output_tokens, r.status_code, r.created_at \
         FROM request_logs r \
         LEFT JOIN providers p ON p.id = r.provider_id",
    );
    let mut conditions: Vec<&str> = Vec::new();
    let mut param_vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = start {
        conditions.push("r.created_at >= ?");
        param_vals.push(Box::new(s.to_string()));
    }
    if let Some(e) = end {
        conditions.push("r.created_at <= ?");
        param_vals.push(Box::new(e.to_string()));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY r.created_at DESC LIMIT 500");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_vals.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_from_iter(param_refs.into_iter()), |row| {
            Ok(RequestLog {
                id: row.get(0)?,
                group_name: row.get(1)?,
                model_name: row.get(2)?,
                provider_name: row.get(3)?,
                latency_ms: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                status_code: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 从库内近期请求日志恢复内存中的指标序列（与单次 push 相同的 24h 截断规则）
pub fn hydrate_aggregator_from_logs(
    conn: &Connection,
    aggregator: &mut MetricsAggregator,
) -> Result<(), String> {
    let samples = load_recent_metric_samples(conn)?;
    for sample in samples {
        aggregator.push(sample);
    }
    Ok(())
}

pub fn load_recent_metric_samples(conn: &Connection) -> Result<Vec<MetricSample>, String> {
    let cutoff = (Utc::now() - chrono::Duration::hours(24))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut stmt = conn
        .prepare(
            "SELECT created_at, latency_ms, input_tokens, output_tokens, total_cost, status_code, \
             COALESCE(cache_creation_input_tokens, 0), COALESCE(cache_read_input_tokens, 0) \
             FROM request_logs WHERE created_at >= ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query([cutoff]).map_err(|e| e.to_string())?;
    let mut samples = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let created: String = row.get(0).map_err(|e| e.to_string())?;
        let at: DateTime<Utc> = DateTime::parse_from_rfc3339(&created)
            .map_err(|e| e.to_string())?
            .with_timezone(&Utc);
        let latency_ms: i64 = row.get(1).map_err(|e| e.to_string())?;
        let input_tokens: i64 = row.get(2).map_err(|e| e.to_string())?;
        let output_tokens: i64 = row.get(3).map_err(|e| e.to_string())?;
        let cost: f64 = row.get(4).map_err(|e| e.to_string())?;
        let status_code: i64 = row.get(5).map_err(|e| e.to_string())?;
        let cache_creation_input_tokens: i64 = row.get(6).map_err(|e| e.to_string())?;
        let cache_read_input_tokens: i64 = row.get(7).map_err(|e| e.to_string())?;
        samples.push(MetricSample {
            at,
            latency_ms,
            input_tokens,
            output_tokens,
            cache_creation_input_tokens,
            cache_read_input_tokens,
            cost,
            is_error: status_code >= 400,
        });
    }
    Ok(samples)
}
