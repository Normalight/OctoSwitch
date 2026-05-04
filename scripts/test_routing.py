#!/usr/bin/env python3
"""Integration tests for OctoSwitch gateway REST API.

Tests the routing endpoints of a running OctoSwitch instance.
Requires OctoSwitch to be running with default groups seeded.

Usage:
  python scripts/test_routing.py
  python scripts/test_routing.py --base-url http://127.0.0.1:8787

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

PASS = 0
FAIL = 0
SKIP = 0


def request_json(
    method: str, path: str, payload: dict | None = None, timeout: int = 5
) -> tuple[int, dict | str]:
    """Make an HTTP request and return (status_code, body)."""
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
            return resp.status, (json.loads(body) if body else {})
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        try:
            return exc.code, json.loads(body)
        except Exception:
            return exc.code, body
    except urllib.error.URLError as exc:
        reason = exc.reason
        if isinstance(reason, ConnectionRefusedError):
            msg = f"OctoSwitch is offline at {BASE_URL} (connection refused)"
        elif isinstance(reason, TimeoutError):
            msg = f"OctoSwitch did not respond in time at {BASE_URL}"
        elif isinstance(reason, socket.gaierror):
            msg = f"OctoSwitch host could not be resolved for {BASE_URL}"
        else:
            msg = f"Failed to reach OctoSwitch at {BASE_URL}: {reason}"
        return 0, {"_offline": True, "message": msg}


def test(name: str) -> None:
    """Decorator-like wrapper to count passes/failures."""

    def decorator(fn):
        def wrapper():
            global PASS, FAIL, SKIP
            try:
                result = fn()
                if result is True:
                    PASS += 1
                    print(f"  PASS  {name}")
                elif result == "SKIP":
                    SKIP += 1
                    print(f"  SKIP  {name}")
                else:
                    FAIL += 1
                    print(f"  FAIL  {name}: {result}")
            except Exception as exc:
                FAIL += 1
                print(f"  FAIL  {name}: {exc}")

        return wrapper

    return decorator


# ---------------------------------------------------------------------------
# Test functions
# ---------------------------------------------------------------------------


@test("GET /healthz returns ok")
def test_healthz():
    status, body = request_json("GET", "/healthz")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    if body.get("ok") is not True:
        return f"expected ok=true, got {body.get('ok')}"
    if body.get("service") != "octoswitch":
        return f"expected service=octoswitch, got {body.get('service')}"
    return True


@test("GET /v1/routing/status returns valid structure")
def test_routing_status():
    status, body = request_json("GET", "/v1/routing/status")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    if "allow_group_member_model_path" not in body:
        return "missing allow_group_member_model_path field"
    if "groups" not in body:
        return "missing groups field"
    if not isinstance(body["groups"], list):
        return f"expected groups to be list, got {type(body['groups']).__name__}"
    return True


@test("GET /v1/models returns list with data")
def test_list_models():
    status, body = request_json("GET", "/v1/models")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    if body.get("object") != "list":
        return f"expected object=list, got {body.get('object')}"
    if "data" not in body:
        return "missing data field"
    if not isinstance(body["data"], list):
        return f"expected data to be list, got {type(body['data']).__name__}"
    return True


@test("GET /v1/plugin/config returns 200")
def test_plugin_config():
    status, body = request_json("GET", "/v1/plugin/config")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    return True


@test("GET /v1/routing/groups/<alias>/members for nonexistent group returns 404")
def test_group_members_nonexistent():
    status, body = request_json("GET", "/v1/routing/groups/NonexistentGroupXYZ/members")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 404:
        return f"expected 404, got {status}"
    return True


@test("POST /v1/routing/groups/<alias>/active-member with invalid member returns 4xx")
def test_set_active_member_invalid():
    status, body = request_json(
        "POST",
        "/v1/routing/groups/NonexistentGroupXYZ/active-member",
        {"member": "nonexistent"},
    )
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status < 400:
        return f"expected 4xx, got {status}"
    return True


@test("GET /v1/models?all=true returns data")
def test_list_models_all():
    status, body = request_json("GET", "/v1/models?all=true")
    if isinstance(body, dict) and body.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    return True


@test("GET /v1/routing/groups/<alias>/members for existing group returns members")
def test_group_members_existing():
    # First get routing status to find an existing group
    status, routing = request_json("GET", "/v1/routing/status")
    if isinstance(routing, dict) and routing.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"routing status failed with {status}"

    groups = routing.get("groups", [])
    if not groups:
        return "SKIP"  # No groups configured, can't test

    alias = groups[0]["alias"]
    alias_encoded = urllib.parse.quote(alias, safe="")
    status, body = request_json(
        "GET", f"/v1/routing/groups/{alias_encoded}/members"
    )
    if status != 200:
        return f"expected 200, got {status}"
    if not isinstance(body, dict):
        return f"expected JSON object, got {type(body).__name__}"
    if body.get("group") != alias:
        return f"expected group={alias}, got {body.get('group')}"
    if "members" not in body:
        return "missing members field"
    if not isinstance(body["members"], list):
        return f"expected members to be list, got {type(body['members']).__name__}"
    return True


@test("POST /v1/routing/groups/<alias>/active-member switches active member")
def test_set_active_member():
    # Get routing status to find an existing group with at least 2 members
    status, routing = request_json("GET", "/v1/routing/status")
    if isinstance(routing, dict) and routing.get("_offline"):
        return "SKIP"
    if status != 200:
        return f"routing status failed with {status}"

    groups = routing.get("groups", [])
    multi_member_groups = [
        g for g in groups if len(g.get("members", [])) >= 2
    ]
    if not multi_member_groups:
        return "SKIP"  # No group with 2+ members

    group = multi_member_groups[0]
    alias = group["alias"]
    alias_encoded = urllib.parse.quote(alias, safe="")

    # Find the currently non-active member to switch to
    members = group["members"]
    inactive = [m for m in members if not m.get("active")]
    if not inactive:
        return "SKIP"  # All members somehow active

    target = inactive[0]["name"]

    status, body = request_json(
        "POST",
        f"/v1/routing/groups/{alias_encoded}/active-member",
        {"member": target},
    )
    if status != 200:
        return f"expected 200, got {status}: {body}"

    if body.get("active_member") != target:
        return f"expected active_member={target}, got {body.get('active_member')}"

    if "model_path" not in body:
        return "missing model_path field"

    expected_path = f"{alias}/{target}"
    if body.get("model_path") != expected_path:
        return f"expected model_path={expected_path}, got {body.get('model_path')}"

    return True


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> int:
    global PASS, FAIL, SKIP

    # Parse --base-url from args
    global BASE_URL
    args = sys.argv[1:]
    i = 0
    while i < len(args):
        if args[i] == "--base-url" and i + 1 < len(args):
            BASE_URL = args[i + 1].rstrip("/")
            i += 2
        else:
            i += 1

    print(f"OctoSwitch Routing Tests")
    print(f"Base URL: {BASE_URL}")
    print()

    test_healthz()
    test_routing_status()
    test_list_models()
    test_plugin_config()
    test_group_members_nonexistent()
    test_set_active_member_invalid()
    test_list_models_all()
    test_group_members_existing()
    test_set_active_member()

    print()
    total = PASS + FAIL + SKIP
    print(f"Results: {PASS} passed, {FAIL} failed, {SKIP} skipped ({total} total)")

    return 0 if FAIL == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
