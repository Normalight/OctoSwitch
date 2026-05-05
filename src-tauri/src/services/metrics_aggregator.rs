use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct MetricSample {
    pub at: DateTime<Utc>,
    #[allow(dead_code)]
    pub latency_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cost: f64,
    #[allow(dead_code)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricKpi {
    pub error_rate: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_consumed_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricPoint {
    pub bucket_time: String,
    pub group_name: String,
    pub provider_name: String,
    pub model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub consumed_tokens: i64,
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
                error_rate: 0.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_read_tokens: 0,
                total_consumed_tokens: 0,
            };
        }

        let total_input_tokens = self.samples.iter().map(|s| s.input_tokens).sum();
        let total_output_tokens = self.samples.iter().map(|s| s.output_tokens).sum();
        let total_cache_read_tokens = self.samples.iter().map(|s| s.cache_read_input_tokens).sum();
        let error_count = self.samples.iter().filter(|s| s.is_error).count() as f64;

        MetricKpi {
            error_rate: error_count / self.samples.len() as f64,
            total_input_tokens,
            total_output_tokens,
            total_cache_read_tokens,
            total_consumed_tokens: total_input_tokens + total_cache_read_tokens + total_output_tokens,
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
        let mut cache_read_tokens = vec![0i64; bucket_count];
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
            cache_read_tokens[idx] += sample.cache_read_input_tokens;
        }

        if !has_any {
            return vec![];
        }

        let mut out = Vec::with_capacity(bucket_count);
        for i in 0..bucket_count {
            let b0 = cutoff + chrono::Duration::seconds((i as i64) * bucket_secs);

            out.push(MetricPoint {
                bucket_time: b0.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                group_name: String::new(),
                provider_name: String::new(),
                model_name: String::new(),
                input_tokens: input_tokens[i],
                output_tokens: output_tokens[i],
                cache_read_tokens: cache_read_tokens[i],
                consumed_tokens: input_tokens[i] + cache_read_tokens[i] + output_tokens[i],
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
        assert_eq!(kpi.total_input_tokens, 100);
        assert_eq!(kpi.total_output_tokens, 220);
        assert_eq!(kpi.total_consumed_tokens, 320);
    }

    #[test]
    fn series_buckets_show_consumed_tokens() {
        let mut aggr = MetricsAggregator::default();
        let now = Utc::now();
        aggr.push(MetricSample {
            at: now - chrono::Duration::seconds(40),
            latency_ms: 10,
            input_tokens: 1,
            output_tokens: 100,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 20,
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
        let with_traffic = pts.iter().find(|p| p.input_tokens > 0 || p.output_tokens > 0);
        assert!(with_traffic.is_some(), "expected a bucket with traffic");
        let p = with_traffic.unwrap();
        assert_eq!(p.input_tokens, 2);
        assert_eq!(p.output_tokens, 300);
        assert_eq!(p.cache_read_tokens, 20);
        assert_eq!(p.consumed_tokens, 322);
    }
}
