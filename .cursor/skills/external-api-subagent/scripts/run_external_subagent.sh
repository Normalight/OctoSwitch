#!/usr/bin/env bash
# 调用本地网关 POST /v1/subagent/run，将提示文件作为单条 user 消息转发至已绑定的上游模型。
# 退出码（稳定约定）: 0 成功且上游 HTTP code < 400；1 参数/环境错误；2 网络/curl 失败；
#                     3 网关 HTTP 非 2xx；4 网关 2xx 但 JSON 中业务 code >= 400。
set -euo pipefail

PROMPT_FILE=""
MODEL_OVERRIDE=""

usage() {
  echo "usage: run_external_subagent.sh --prompt-file PATH [--model NAME]" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prompt-file)
      PROMPT_FILE="${2:-}"
      shift 2
      ;;
    --model)
      MODEL_OVERRIDE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 1
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$PROMPT_FILE" || ! -f "$PROMPT_FILE" ]]; then
  echo "EXTERNAL_SUBAGENT_ERROR=missing_or_unreadable_prompt_file" >&2
  exit 1
fi

BASE_URL="${EXTERNAL_SUBAGENT_BASE_URL:-http://127.0.0.1:8787}"
BASE_URL="${BASE_URL%/}"
TIMEOUT_SEC="${EXTERNAL_SUBAGENT_TIMEOUT_SEC:-120}"
MODEL="${MODEL_OVERRIDE:-${EXTERNAL_SUBAGENT_MODEL:-}}"

if [[ -z "$MODEL" ]]; then
  echo "EXTERNAL_SUBAGENT_ERROR=missing_model_set_EXTERNAL_SUBAGENT_MODEL_or_pass_--model" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "EXTERNAL_SUBAGENT_ERROR=curl_not_found" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "EXTERNAL_SUBAGENT_ERROR=python3_required_for_json_payload" >&2
  exit 1
fi

BODY="$(
  PROMPT_FILE="$PROMPT_FILE" MODEL="$MODEL" python3 -c '
import json, os, pathlib
p = pathlib.Path(os.environ["PROMPT_FILE"]).read_text(encoding="utf-8")
print(json.dumps({"model": os.environ["MODEL"], "messages": [{"role": "user", "content": p}]}))
')"

TMP_HDR="$(mktemp)"
TMP_BODY="$(mktemp)"
cleanup() { rm -f "$TMP_HDR" "$TMP_BODY"; }
trap cleanup EXIT

HTTP_CODE=""
if ! HTTP_CODE="$(curl -sS -m "$TIMEOUT_SEC" -X POST "$BASE_URL/v1/subagent/run" \
  -H "Content-Type: application/json" \
  ${EXTERNAL_SUBAGENT_API_KEY:+-H "Authorization: Bearer ${EXTERNAL_SUBAGENT_API_KEY}"} \
  -D "$TMP_HDR" \
  -o "$TMP_BODY" \
  -w "%{http_code}" \
  --data-binary "$BODY")"; then
  echo "EXTERNAL_SUBAGENT_ERROR=curl_failed" >&2
  echo "CORRELATION_ID=" >&2
  exit 2
fi

RAW_JSON="$(cat "$TMP_BODY" || true)"

extract_correlation_id() {
  echo "$1" | python3 -c 'import json,sys; 
try:
  d=json.load(sys.stdin); print(d.get("correlation_id") or "")
except Exception:
  print("")' 2>/dev/null || true
}

CID="$(extract_correlation_id "$RAW_JSON")"
echo "CORRELATION_ID=${CID}" >&2

if [[ "$HTTP_CODE" -lt 200 || "$HTTP_CODE" -ge 300 ]]; then
  echo "EXTERNAL_SUBAGENT_ERROR=gateway_http_${HTTP_CODE}" >&2
  echo "$RAW_JSON" >&2
  exit 3
fi

# 解析业务层 code（上游 HTTP 状态或网关错误映射）
APP_CODE="$(
  echo "$RAW_JSON" | python3 -c 'import json,sys
try:
  d=json.load(sys.stdin); c=d.get("code")
  print(int(c) if c is not None else 0)
except Exception:
  print(-1)
' 2>/dev/null || echo "-1")"

if [[ "$APP_CODE" -lt 0 ]]; then
  echo "EXTERNAL_SUBAGENT_ERROR=invalid_response_json" >&2
  echo "$RAW_JSON" >&2
  exit 3
fi

echo "$RAW_JSON"
if [[ "$APP_CODE" -ge 400 ]]; then
  echo "EXTERNAL_SUBAGENT_ERROR=upstream_or_gateway_reason_http_${APP_CODE}" >&2
  exit 4
fi

exit 0
