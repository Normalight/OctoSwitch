---
name: external-api-subagent
description: Call non-default API models as external subagents through a gateway endpoint, then merge their results into the current task. Use when the user asks to use another model/provider, compare multiple model outputs, or delegate review/drafting to external API models.
---

# External API Subagent

## Status

**Runnable.** 脚本会请求网关 `POST /v1/subagent/run`；响应 JSON 含数值字段 `code`（HTTP 语义）与 `correlation_id`。出站 HTTP 默认代理为 `http://192.168.1.11:7897`（见网关 `MG_HTTP_PROXY`，可覆盖）。

## Purpose

Use this skill when the user wants Claude Code to invoke other API models (for example OpenAI-compatible or Anthropic-compatible models exposed by your gateway).

This skill treats external model calls as "virtual subagents":

1. Build a focused task prompt.
2. Call the external model via script.
3. Capture and summarize output.
4. Continue implementation or review with clear attribution.

## Requirements

Set these environment variables before using:

- `EXTERNAL_SUBAGENT_BASE_URL` — 网关根地址（默认 `http://127.0.0.1:8787`，**不要**带 `/v1/chat/completions`）
- `EXTERNAL_SUBAGENT_MODEL` — 网关侧已绑定的逻辑模型名（可用 `--model` 覆盖）
- `EXTERNAL_SUBAGENT_API_KEY` — 可选；若设置则脚本会带 `Authorization: Bearer …`（仅当网关在后续版本要求鉴权时使用）

Optional:

- `EXTERNAL_SUBAGENT_TIMEOUT_SEC` (default `120`)

脚本依赖：`curl`、`python3`（用于安全构造 JSON 请求体）。

## Workflow

Copy this checklist and execute in order:

```text
External API Subagent Checklist
- [ ] Confirm task can be delegated to an external model
- [ ] Write a narrow, verifiable prompt for the external model
- [ ] Call script: .cursor/skills/external-api-subagent/scripts/run_external_subagent.sh
- [ ] Validate output quality and consistency with repo constraints
- [ ] Merge useful results and cite as external model output
```

## Prompt Template

Use this structure for stable results:

```text
You are an external specialist model.
Task: <single concrete task>
Constraints:
- <constraint 1>
- <constraint 2>
Output format:
- <exact format>
Repository context:
- <minimal relevant context>
```

## Command

```bash
bash ".cursor/skills/external-api-subagent/scripts/run_external_subagent.sh" \
  --prompt-file "/tmp/external-subagent-prompt.txt" \
  --model "optional-model-override"
```

If `--model` is omitted, script uses `EXTERNAL_SUBAGENT_MODEL`.

## Script exit codes & stderr

| Code | Meaning |
|------|---------|
| 0 | 成功：`code` &lt; 400 |
| 1 | 参数/环境错误（`EXTERNAL_SUBAGENT_ERROR=…`） |
| 2 | 网络 / curl 失败 |
| 3 | 网关 HTTP 非 2xx |
| 4 | 网关 2xx 但 JSON `code` ≥ 400（上游或业务错误） |

stderr 始终包含一行 `CORRELATION_ID=<uuid 或空>`，便于与网关日志对齐。

## Output Handling Rules

- Treat external output as draft input, not source of truth.
- Re-check security, compatibility, and repo conventions before applying.
- If output is low quality, tighten prompt and retry once with stricter format.
- Keep final response explicit: which parts came from external model reasoning.

## Examples

- [Basic usage](examples/basic.md)
