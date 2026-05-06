#!/usr/bin/env python3
"""Test OctoSwitch SSE streaming for Anthropic /v1/messages.

Tests:
1. stream=true (boolean)
2. stream as object (if Anthropic SDK sends this)
3. Response timing per chunk
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request

BASE_URL = os.environ.get("OCTOSWITCH_BASE_URL", "http://127.0.0.1:8787").rstrip("/")

MODEL = os.environ.get("TEST_MODEL", "Sonnet")

def test_sse_stream(label: str, payload: dict) -> bool:
    """Send a streaming request and measure chunk timing."""
    url = f"{BASE_URL}/v1/messages"
    data = json.dumps(payload).encode("utf-8")

    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "text/event-stream")

    print(f"\n{'='*60}")
    print(f"Test: {label}")
    print(f"stream field: {payload.get('stream')!r}")
    print(f"{'='*60}")

    try:
        resp = urllib.request.urlopen(req, timeout=30)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        print(f"  HTTP {e.code}: {body[:500]}")
        return False
    except urllib.error.URLError as e:
        print(f"  Connection failed: {e.reason}")
        return False

    content_type = resp.headers.get("Content-Type", "")
    print(f"  Content-Type: {content_type}")

    if "text/event-stream" not in content_type:
        # Non-streaming response — read all at once
        body = resp.read().decode("utf-8", errors="replace")
        print(f"  NON-STREAMING response ({len(body)} bytes)")
        print(f"  Body preview: {body[:500]}")
        return False

    # Read SSE chunks with timing
    chunks = []
    chunk_times = []
    buffer = b""
    start = time.time()

    while True:
        try:
            chunk = resp.read1(4096)  # Read up to 4KB, don't wait for full buffer
        except Exception:
            chunk = resp.read(4096)

        if not chunk:
            break

        now = time.time()
        chunks.append(chunk)
        chunk_times.append(now)

        buffer += chunk
        # Count SSE messages in this chunk
        sse_count = buffer.count(b"\n\n") + buffer.count(b"\r\n\r\n")
        print(f"  chunk #{len(chunks):3d} | {len(chunk):5d} bytes | +{now - start:.3f}s | ~{sse_count} SSE msgs in buffer")

        if len(chunks) >= 100:
            print(f"  ... stopped after 100 chunks")
            break

    elapsed = time.time() - start
    total_bytes = sum(len(c) for c in chunks)

    print(f"\n  Summary: {len(chunks)} chunks, {total_bytes} bytes, {elapsed:.2f}s")

    if len(chunks) <= 2:
        print(f"  WARNING: Only {len(chunks)} chunks — looks like non-streaming or heavily buffered!")
        return False

    # Check chunk timing pattern
    if len(chunks) > 2:
        deltas = [chunk_times[i] - chunk_times[i-1] for i in range(1, len(chunks))]
        avg_gap = sum(deltas) / len(deltas)
        max_gap = max(deltas)
        print(f"  Inter-chunk gaps: avg={avg_gap*1000:.0f}ms max={max_gap*1000:.0f}ms")

        if max_gap > 5.0:
            print(f"  WARNING: Large gaps detected — may indicate buffering issue")

    return True


def main():
    # Test 1: Standard boolean streaming
    ok1 = test_sse_stream("Boolean stream=true", {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Say hello in exactly 5 words."}],
        "stream": True,
        "max_tokens": 50,
    })

    # Test 2: Object-format stream (newer Anthropic API)
    ok2 = test_sse_stream("Object stream", {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Say hello in exactly 5 words."}],
        "stream": {"type": "text"},
        "max_tokens": 50,
    })

    # Test 3: No stream field (non-streaming baseline)
    ok3 = test_sse_stream("No stream field (non-streaming)", {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Say hello in exactly 5 words."}],
        "max_tokens": 50,
    })

    print("\n" + "="*60)
    print("RESULTS:")
    print(f"  stream=true (boolean):    {'PASS' if ok1 else 'FAIL'}")
    print(f"  stream={{type:text}}:      {'PASS' if ok2 else 'FAIL — object format not detected as streaming!'}")
    print(f"  no stream field:          {'PASS (correctly non-streaming)' if not ok3 else 'UNEXPECTED STREAMING'}")

    if not ok2:
        print("\n  ROOT CAUSE: stream as object is NOT detected by as_bool()!")
        print("  Fix: check is_object() too in handle_anthropic_messages")

    return 0 if (ok1 and ok2) else 1

if __name__ == "__main__":
    sys.exit(main())
