use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct CircuitBreakerService {
    states: HashMap<String, BreakerState>,
}

#[derive(Clone)]
struct BreakerState {
    open_until: Option<Instant>,
    failures: u32,
}

impl Default for BreakerState {
    fn default() -> Self {
        Self {
            open_until: None,
            failures: 0,
        }
    }
}

impl CircuitBreakerService {
    pub fn is_open(&self, provider_id: &str) -> bool {
        self.states
            .get(provider_id)
            .and_then(|s| s.open_until)
            .map(|t| t > Instant::now())
            .unwrap_or(false)
    }

    pub fn mark_success(&mut self, provider_id: &str) {
        let state = self.states.entry(provider_id.to_string()).or_default();
        let was_open = state.open_until.is_some() || state.failures > 0;
        state.failures = 0;
        state.open_until = None;
        if was_open {
            log::info!(
                "[{}] provider '{}' circuit breaker closed (healthy)",
                crate::log_codes::CB_CLOSED,
                provider_id
            );
        }
    }

    pub fn mark_failure(&mut self, provider_id: &str) {
        let state = self.states.entry(provider_id.to_string()).or_default();
        state.failures += 1;
        if state.failures >= 3 {
            state.open_until = Some(Instant::now() + Duration::from_secs(20));
            log::warn!(
                "[{}] provider '{}' circuit breaker OPEN after {} failures",
                crate::log_codes::CB_OPEN,
                provider_id,
                state.failures
            );
        } else {
            log::warn!(
                "[{}] provider '{}' failure {}/3",
                crate::log_codes::CB_OPEN,
                provider_id,
                state.failures
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_open_returns_false_initially() {
        let service = CircuitBreakerService::default();
        assert!(!service.is_open("any_provider"));
    }

    #[test]
    fn test_mark_failure_opens_after_three() {
        let mut service = CircuitBreakerService::default();
        service.mark_failure("p1");
        assert!(!service.is_open("p1"));
        service.mark_failure("p1");
        assert!(!service.is_open("p1"));
        service.mark_failure("p1");
        assert!(service.is_open("p1"));
    }

    #[test]
    fn test_mark_failure_below_three_stays_closed() {
        let mut service = CircuitBreakerService::default();
        service.mark_failure("p1");
        service.mark_failure("p1");
        assert!(!service.is_open("p1"));
    }

    #[test]
    fn test_mark_success_closes_circuit() {
        let mut service = CircuitBreakerService::default();
        service.mark_failure("p1");
        service.mark_failure("p1");
        service.mark_failure("p1");
        assert!(service.is_open("p1"));
        service.mark_success("p1");
        assert!(!service.is_open("p1"));
    }

    #[test]
    fn test_mark_success_does_nothing_on_healthy() {
        let mut service = CircuitBreakerService::default();
        service.mark_success("p1");
        assert!(!service.is_open("p1"));
    }

    #[test]
    fn test_multiple_providers_independent() {
        let mut service = CircuitBreakerService::default();
        service.mark_failure("p_a");
        service.mark_failure("p_a");
        service.mark_failure("p_a");
        assert!(service.is_open("p_a"));
        assert!(!service.is_open("p_b"));
    }

    #[test]
    fn test_circuit_opens_after_cooldown() {
        let mut service = CircuitBreakerService::default();
        service.mark_failure("p1");
        service.mark_failure("p1");
        service.mark_failure("p1");
        assert!(service.is_open("p1"));
        service.mark_failure("p1");
        assert!(service.is_open("p1"));
    }
}
