# External API Subagent Integration Plan

## Scope

Turn the current planned skill into runnable capability after gateway software exists.

## Milestones

### M1: Gateway MVP available

- Local service starts with stable lifecycle.
- Provider model: `BaseURL + APIKey -> Models[]`.
- OpenAI-compatible and Anthropic-compatible paths respond successfully.
- Security whitelist is enforced at runtime.

### M2: Invocation contract

- Define request/response schema for external subagent calls.
- Add correlation id and audit logging.
- Provide deterministic error codes for retry/abort decisions.

### M3: Script implementation

- Replace placeholder shell script with real caller logic.
- Support model override and timeout handling.
- Add human-readable and machine-readable output modes.

### M4: Skill hardening

- Add examples for review, drafting, and compare-two-model tasks.
- Add validation checklist before applying external outputs.
- Add fallback behavior when gateway unavailable.

## Verification Checklist

- [ ] Smoke test OpenAI-compatible route
- [ ] Smoke test Anthropic-compatible route
- [ ] Verify whitelist block on non-approved host
- [ ] Verify audit log is written for success/failure
- [ ] Verify skill examples run end-to-end

## Exit Criteria

The skill can invoke external models reliably, and all outputs are auditable and bounded by security policy.
