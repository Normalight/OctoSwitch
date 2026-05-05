---
name: reload
description: Reload the OctoSwitch plugin distribution to sync routing rules and skills.
allowed-tools: Bash(python scripts/octoswitch_routing.py*), Read
disable-model-invocation: true
---

# /reload

Use this skill when the user wants to reload the OctoSwitch plugin distribution — for example after changing routing rules, task preferences, or skill definitions.

Compatibility forms:

- `/reload`
- `/octoswitch:reload`

When exported as a plugin artifact, publish this command under the `octoswitch` namespace.

## Goal

Trigger a full plugin distribution rebuild and sync so that cc-switch picks up the latest routing rules, task routes, and skill definitions.

## Direct execution behavior

When invoked as a project-local command:

1. Run:

```bash
python scripts/octoswitch_routing.py reload
```

2. Parse the returned JSON and reply with a short confirmation.

## Output contract

Keep the result short.

Preferred output:

```text
Plugin reloaded.
  Route: POST /v1/plugin/reload
  Status: <status from API>
  Copied files: <count>
  Removed files: <count>
  Preserved files: <count>
  Commands run: python scripts/octoswitch_routing.py reload
  Test results: N/A
  Unresolved risks: none
```

## Failure handling

If the API is unavailable:

- report `OctoSwitch: offline`
- include the returned offline message
- mention `OCTOSWITCH_BASE_URL`
- mention the default address `http://127.0.0.1:8787`

Do not invent sync results.
