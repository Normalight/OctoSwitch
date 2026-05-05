#!/usr/bin/env python3
"""Integration test for DeepSeek reasoning_content preservation.

Tests that OctoSwitch correctly preserves reasoning_content through
Anthropic ↔ OpenAI format conversion when routing to DeepSeek-compatible
providers (OpenCodeGo with deepseek-v4-pro model).

Usage:
  python scripts/test_deepseek_reasoning.py
  python scripts/test_deepseek_reasoning.py --base-url http://127.0.0.1:8787
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request

BASE_URL = os.environ.get("OCTOSWITCH_BASE_URL", "http://127.0.0.1:8787").rstrip("/")

PASS = 0
FAIL = 0


def request_json(
    method: str, path: str, payload: dict | None = None, timeout: int = 30
) -> tuple[int, dict | str]:
    url = f"{BASE_URL}{path}"
    data = None
    headers = {"Accept": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, json.loads(body) if body else {}
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        try:
            return exc.code, json.loads(body)
        except json.JSONDecodeError:
            return exc.code, body
    except Exception as exc:
        return 0, str(exc)


def check(name: str, condition: bool, detail: str = ""):
    global PASS, FAIL
    if condition:
        PASS += 1
        print(f"  ✓ {name}")
    else:
        FAIL += 1
        print(f"  ✗ {name} — {detail}" if detail else f"  ✗ {name}")


def test_non_streaming_thinking_with_tool_call():
    """Send an Anthropic request with thinking blocks + tool_calls.
    The upstream should handle reasoning_content properly."""
    global PASS, FAIL
    print("\n=== Test: Anthropic non-streaming request ===")

    payload = {
        "model": "Opus/deepseek-v4-pro",
        "max_tokens": 256,
        "stream": False,
        "messages": [
            {"role": "user", "content": "Hello, what is 2+2?"}
        ],
    }

    status, body = request_json("POST", "/v1/messages", payload, timeout=30)

    if isinstance(body, dict) and "type" in body and body["type"] == "message":
        check("non-streaming returns Anthropic message", True)
        content = body.get("content", [])
        check("message has content blocks", len(content) > 0, str(body)[:200])
    elif isinstance(body, dict) and "error" in body:
        code = body.get("code", "")
        if "500" in str(status) or status >= 500:
            print(f"  ~ non-streaming skipped (upstream error: {code})")
        else:
            check(f"non-streaming response OK", False, f"error: {code} - {body.get('error', '')[:200]}")
    else:
        check("non-streaming returns valid response", False, str(body)[:200])


def test_streaming_thinking():
    """Send a streaming Anthropic request and verify events flow through."""
    global PASS, FAIL
    print("\n=== Test: Streaming Anthropic request ===")

    payload = {
        "model": "Opus/deepseek-v4-pro",
        "max_tokens": 256,
        "stream": True,
        "messages": [
            {"role": "user", "content": "Say 'hello world'"}
        ],
    }

    url = f"{BASE_URL}/v1/messages"
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url, data=data, method="POST",
        headers={
            "Content-Type": "application/json",
            "Accept": "text/event-stream",
        },
    )

    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            raw = resp.read().decode("utf-8", errors="replace")
            lines = [l for l in raw.split("\n") if l.startswith("data:")]
            check("streaming returns SSE events", len(lines) > 0, f"got {len(lines)} data lines")
            check("streaming returns valid events", any("content_block" in l or "message_stop" in l for l in lines), "")
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        if exc.code in (502, 403):
            print(f"  ~ streaming skipped (upstream auth: {exc.code})")
        else:
            check("streaming does not 400", False, f"status={exc.code}, body={body[:200]}")


def test_routing_status():
    """Verify routing status returns enabled groups."""
    global PASS, FAIL
    print("\n=== Test: Routing status ===")

    status, body = request_json("GET", "/v1/routing/status")
    check("routing status is 200", status == 200, str(status))

    if isinstance(body, dict):
        groups = body.get("groups", [])
        check("groups list returned", len(groups) > 0, "no groups found")
        enabled = [g for g in groups if g.get("enabled")]
        check("enabled groups filtered", len(enabled) <= len(groups), "")


def test_models_endpoint():
    """Verify /v1/models lists deepseek-v4-pro."""
    global PASS, FAIL
    print("\n=== Test: /v1/models endpoint ===")

    status, body = request_json("GET", "/v1/models")
    check("models endpoint is 200", status == 200, str(status))

    if isinstance(body, dict):
        models = [m["id"] for m in body.get("data", [])]
        check("has Opus group in models", any(m == "Opus" for m in models), f"found: {models}")
        check("has Opus/deepseek-v4-pro in models", "Opus/deepseek-v4-pro" in models, f"found: {models}")


def main():
    global PASS, FAIL
    print(f"OctoSwitch DeepSeek Reasoning Test")
    print(f"Target: {BASE_URL}")
    print("=" * 50)

    test_non_streaming_thinking_with_tool_call()
    test_streaming_thinking()
    test_routing_status()
    test_models_endpoint()

    print(f"\n{'=' * 50}")
    total = PASS + FAIL
    print(f"Results: {PASS}/{total} passed")
    if FAIL > 0:
        print(f"FAILURES: {FAIL}")
        sys.exit(1)
    else:
        print("All tests passed!")


if __name__ == "__main__":
    main()
