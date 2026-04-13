# Claude routing skills source

This folder is the tracked source of the Claude Code routing skills used with OctoSwitch.

We intentionally keep `.claude/` ignored in Git, so the canonical editable copies live here and are installed into `.claude/skills/` when needed.

## Purpose

These skills make Claude Code the interaction layer while OctoSwitch acts as the routing control plane.

That means:

- Claude Code issues human-friendly commands
- OctoSwitch exposes routing state and active-member switching
- explicit `group/member` routes can be used for task-scoped delegation
- future automatic routing can build on the same control surface

## Prompt design principles

The skill prompts in this folder are aligned around the same rules:

- short route-aware control commands
- explicit execution contracts for delegated work
- fixed output shapes for worker responses
- no silent route guessing
- clear failure messages when a group or member is missing
- first-run onboarding before automatic delegation is trusted

## Skill map

### Executable now

- `show-routing` — inspect current routing status
- `route-activate` — switch any group to a chosen member
- `delegate` — delegate work to `executor`, `executor/<member>`, or an explicit route

### Compatibility alias

- `delegate-to` — legacy alias for explicit routing; prefer `/delegate --to ...`

### Design-stage skills

- `task-route` — define recommended routes for task kinds
- `delegate-auto` — let the routing layer propose subagent count, roles, and routes

## Shared helper script

```text
scripts/octoswitch_routing.py
```

## Default OctoSwitch endpoint

```text
http://127.0.0.1:8787
```

Override with:

```text
OCTOSWITCH_BASE_URL=http://127.0.0.1:8787
```

## Install into project-local Claude Code

Recommended installer:

```powershell
.\scripts\install_claude_skills.ps1
```

This copies the tracked source folders into:

```text
.claude/skills/
```

## Recommended command surface

Primary commands:

- `/show-routing`
- `/route-activate <group> <member>`
- `/delegate ...`

Extended / design-stage commands:

- `/delegate --to <group>|<group/member> <task>`
- `/task-route ...`
- `/delegate-auto ...`
