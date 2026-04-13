---
name: show-routing
description: Show OctoSwitch routing status for planner/executor groups and all group members.
allowed-tools: Bash(python scripts/octoswitch_routing.py*), Read
disable-model-invocation: true
---

# /show-routing

Use this skill when the user wants to inspect OctoSwitch routing state before choosing a route, switching an active member, or delegating work.

Compatibility forms:

- `/show-routing`
- `/octoswitch:show-routing`

When exported as a plugin artifact, publish this command under the `octoswitch` namespace.

## Goal

Provide a concise routing snapshot that is useful for:

- checking whether OctoSwitch is reachable
- verifying whether `group/member` routing is enabled
- seeing which member is active for each group
- confirming what routes are available before `/delegate` or `/route-activate`

## Direct execution behavior

When invoked as a project-local command:

1. Run:

```bash
python scripts/octoswitch_routing.py status
```

2. Summarize in this order:
   - OctoSwitch availability
   - whether `group/member` routing is enabled
   - key groups such as `planner` and `executor` if present
   - remaining groups and their members

## Output contract

Keep the result short, route-oriented, and easy to scan.

Preferred shape:

```text
OctoSwitch: online
Group/member routing: enabled

planner -> opus
  members: opus, sonnet

executor -> sonnet
  members: sonnet, opus, haiku
```

## Failure handling

If the API is unavailable:

- report `OctoSwitch: offline`
- include the returned offline message
- mention `OCTOSWITCH_BASE_URL`
- mention the default address `http://127.0.0.1:8787`

Do not invent routing state.
