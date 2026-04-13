use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct MetricSample {
    pub at: DateTime<Utc>,
    #[allow(dead_code)]
    pub latency_ms: i64,
    #[allow(dead_code)]
    pub input_tokens: i64,
    #[allow(dead_code)]
    pub output_tokens: i64,
    #[allow(dead_code)]
    pub cache_creation_input_tokens: i64,
    #[allow(dead_code)]
    pub cache_read_input_tokens: i64,
    #[allow(dead_code)]
    pub cost: f64,
    #[allow(dead_code)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricKpi {
    pub avg_qps: f64,
    pub avg_tps: f64,
    pub error_rate: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricPoint {
    pub bucket_time: String,
    pub qps: f64,
    pub tps: f64,
    pub cost: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
}

#[derive(Default)]
pub struct MetricsAggregator {
    samples: VecDeque<MetricSample>,
}

impl MetricsAggregator {
    pub fn push(&mut self, sample: MetricSample) {
        self.samples.push_back(sample);
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        while let Some(front) = self.samples.front() {
            if front.at < cutoff {
                let _ = self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    #[cfg(test)]
    pub fn kpi(&self) -> MetricKpi {
        if self.samples.is_empty() {
            return MetricKpi {
                avg_qps: 0.0,
                avg_tps: 0.0,
                error_rate: 0.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_creation_tokens: 0,
                total_cache_read_tokens: 0,
                total_cost: 0.0,
            };
        }

        let total_input_tokens = self.samples.iter().map(|s| s.input_tokens).sum();
        let total_output_tokens = self.samples.iter().map(|s| s.output_tokens).sum();
        let total_cost = self.samples.iter().map(|s| s.cost).sum();
        let error_count = self.samples.iter().filter(|s| s.is_error).count() as f64;

        MetricKpi {
            avg_qps: self.samples.len() as f64 / 86400.0,
            avg_tps: total_output_tokens as f64 / 86400.0,
            error_rate: error_count / self.samples.len() as f64,
            total_input_tokens,
            total_output_tokens,
            total_cache_creation_tokens: self
                .samples
                .iter()
                .map(|s| s.cache_creation_input_tokens)
                .sum(),
            total_cache_read_tokens: self.samples.iter().map(|s| s.cache_read_input_tokens).sum(),
            total_cost,
        }
    }

    /// 按固定时长分桶：每桶的 QPS / TPS 由**桶内观测到的请求**换算（桶长秒的均值），无请求则为 0。
    #[cfg(test)]
    pub fn series(&self, window: &str) -> Vec<MetricPoint> {
        let minutes: i64 = match window {
            "5m" => 5,
            "1h" => 60,
            _ => 24 * 60,
        };
        let bucket_secs: i64 = match window {
            "5m" => 30,
            "1h" => 300,
            _ => 3600,
        };
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes);
        let now = Utc::now();

        let samples_in_win: Vec<MetricSample> = self
            .samples
            .iter()
            .filter(|s| s.at >= cutoff && s.at <= now)
            .cloned()
            .collect();
        Self::build_series_from_samples(&samples_in_win, cutoff, now, bucket_secs)
    }

    /// 由样本构造分桶趋势（与 [`Self::series`] 同一套换算规则），供库内区间查询复用。
    #[allow(dead_code)]
    pub fn build_series_from_samples(
        samples: &[MetricSample],
        cutoff: DateTime<Utc>,
        now: DateTime<Utc>,
        bucket_secs: i64,
    ) -> Vec<MetricPoint> {
        if bucket_secs <= 0 || now <= cutoff {
            return vec![];
        }

        let total_span_secs = (now - cutoff).num_seconds();
        let bucket_count = ((total_span_secs + bucket_secs - 1) / bucket_secs) as usize;
        if bucket_count == 0 {
            return vec![];
        }

        let mut req_count = vec![0usize; bucket_count];
        let mut input_tokens = vec![0i64; bucket_count];
        let mut output_tokens = vec![0i64; bucket_count];
        let mut cache_creation_tokens = vec![0i64; bucket_count];
        let mut cache_read_tokens = vec![0i64; bucket_count];
        let mut cost_sum = vec![0f64; bucket_count];
        let mut has_any = false;

        for sample in samples {
            if sample.at < cutoff || sample.at > now {
                continue;
            }
            has_any = true;
            let delta_secs = (sample.at - cutoff).num_seconds();
            let mut idx = (delta_secs / bucket_secs) as usize;
            if idx >= bucket_count {
                idx = bucket_count - 1;
            }

            req_count[idx] += 1;
            input_tokens[idx] += sample.input_tokens;
            output_tokens[idx] += sample.output_tokens;
            cache_creation_tokens[idx] += sample.cache_creation_input_tokens;
            cache_read_tokens[idx] += sample.cache_read_input_tokens;
            cost_sum[idx] += sample.cost;
        }

        if !has_any {
            return vec![];
        }

        let mut out = Vec::with_capacity(bucket_count);
        for i in 0..bucket_count {
            let b0 = cutoff + chrono::Duration::seconds((i as i64) * bucket_secs);
            let b1_raw = b0 + chrono::Duration::seconds(bucket_secs);
            let b1 = b1_raw.min(now);
            let dur_secs = (b1 - b0).num_seconds().max(1) as f64;

            out.push(MetricPoint {
                bucket_time: b0.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                qps: req_count[i] as f64 / dur_secs,
                tps: output_tokens[i] as f64 / dur_secs,
                cost: cost_sum[i],
                input_tokens: input_tokens[i],
                output_tokens: output_tokens[i],
                cache_creation_tokens: cache_creation_tokens[i],
                cache_read_tokens: cache_read_tokens[i],
            });
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{MetricSample, MetricsAggregator};

    #[test]
    fn kpi_is_non_zero_after_samples() {
        let mut aggr = MetricsAggregator::default();
        aggr.push(MetricSample {
            at: Utc::now(),
            latency_ms: 120,
            input_tokens: 100,
            output_tokens: 220,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost: 0.002,
            is_error: false,
        });
        let kpi = aggr.kpi();
        assert!(kpi.avg_qps > 0.0);
        assert!(kpi.avg_tps > 0.0);
        assert_eq!(kpi.total_input_tokens, 100);
        assert_eq!(kpi.total_output_tokens, 220);
    }

    #[test]
    fn series_buckets_average_qps_tps_from_samples() {
        let mut aggr = MetricsAggregator::default();
        let now = Utc::now();
        aggr.push(MetricSample {
            at: now - chrono::Duration::seconds(40),
            latency_ms: 10,
            input_tokens: 1,
            output_tokens: 100,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost: 0.001,
            is_error: false,
        });
        aggr.push(MetricSample {
            at: now - chrono::Duration::seconds(35),
            latency_ms: 10,
            input_tokens: 1,
            output_tokens: 200,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cost: 0.002,
            is_error: false,
        });
        let pts = aggr.series("5m");
        assert!(!pts.is_empty());
        let with_traffic = pts.iter().find(|p| p.qps > 0.0 || p.tps > 0.0);
        assert!(with_traffic.is_some(), "expected a bucket with traffic");
        let p = with_traffic.unwrap();
        assert!((p.qps - 2.0 / 30.0).abs() < 1e-6);
        assert!((p.tps - 300.0 / 30.0).abs() < 1e-6);
    }
}
