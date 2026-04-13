#!/usr/bin/env python3
"""Small helper for Claude Code routing skills.

Usage:
  python scripts/octoswitch_routing.py health
  python scripts/octoswitch_routing.py status
  python scripts/octoswitch_routing.py members <group>
  python scripts/octoswitch_routing.py activate <group> <member>

Environment:
  OCTOSWITCH_BASE_URL  Optional. Defaults to http://127.0.0.1:8787
"""

from __future__ import annotations

import json
import os
import socket
import sys
import urllib.error
import urllib.parse
import urllib.request


BASE_URL = os.environ.get("OCTOSWITCH_BASE_URL", "http://127.0.0.1:8787").rstrip("/")


def format_connection_issue(exc: urllib.error.URLError) -> str:
    reason = exc.reason
    if isinstance(reason, ConnectionRefusedError):
        return (
            f"OctoSwitch is offline at {BASE_URL} "
            "(connection refused; service may not be started)"
        )
    if isinstance(reason, TimeoutError):
        return f"OctoSwitch did not respond in time at {BASE_URL}"
    if isinstance(reason, socket.gaierror):
        return f"OctoSwitch host could not be resolved for {BASE_URL}"
    return f"Failed to reach OctoSwitch at {BASE_URL}: {reason}"


def offline_status(exc: urllib.error.URLError) -> dict:
    return {
        "online": False,
        "status": "offline",
        "base_url": BASE_URL,
        "message": format_connection_issue(exc),
        "hints": [
            "Check whether OctoSwitch is running.",
            "Check OCTOSWITCH_BASE_URL.",
            "Default local address is http://127.0.0.1:8787.",
        ],
    }


def offline_status_from_message(message: str) -> dict:
    return {
        "online": False,
        "status": "offline",
        "base_url": BASE_URL,
        "message": message,
        "hints": [
            "Check whether OctoSwitch is running.",
            "Check OCTOSWITCH_BASE_URL.",
            "Default local address is http://127.0.0.1:8787.",
        ],
    }


def request_json(method: str, path: str, payload: dict | None = None) -> dict:
    url = f"{BASE_URL}{path}"
    data = None
    headers = {"Accept": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            body = resp.read().decode("utf-8")
            return json.loads(body) if body else {}
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        detail = body
        try:
            parsed = json.loads(body)
            detail = json.dumps(parsed, ensure_ascii=False)
        except Exception:
            pass
        raise SystemExit(f"HTTP {exc.code} {exc.reason}: {detail}") from exc
    except urllib.error.URLError as exc:
        raise SystemExit(format_connection_issue(exc)) from exc


def print_json(obj: dict) -> None:
    print(json.dumps(obj, ensure_ascii=False, indent=2))


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__.strip())
        return 1

    cmd = argv[1]
    if cmd == "health":
        try:
            print_json(request_json("GET", "/healthz"))
        except SystemExit as exc:
            message = str(exc)
            if message.startswith("OctoSwitch ") or message.startswith("Failed to reach OctoSwitch"):
                print_json(offline_status_from_message(message))
                return 0
            raise
        return 0
    if cmd == "status":
        try:
            print_json(request_json("GET", "/v1/routing/status"))
        except SystemExit as exc:
            message = str(exc)
            if message.startswith("OctoSwitch ") or message.startswith("Failed to reach OctoSwitch"):
                print_json(offline_status_from_message(message))
                return 0
            raise
        return 0
    if cmd == "members":
        if len(argv) != 3:
            raise SystemExit("Usage: members <group>")
        group = urllib.parse.quote(argv[2], safe="")
        print_json(request_json("GET", f"/v1/routing/groups/{group}/members"))
        return 0
    if cmd == "activate":
        if len(argv) != 4:
            raise SystemExit("Usage: activate <group> <member>")
        group = urllib.parse.quote(argv[2], safe="")
        member = argv[3]
        print_json(
            request_json(
                "POST",
                f"/v1/routing/groups/{group}/active-member",
                {"member": member},
            )
        )
        return 0

    raise SystemExit(f"Unknown command: {cmd}")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
