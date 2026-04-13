# Install these skills into `.claude/skills`

This repository keeps Claude routing skills in the tracked `skills/` folder.

We intentionally do not track `.claude/` in Git.

## Recommended install flow

From the project root:

```powershell
.\scripts\install_claude_skills.ps1
```

If you only want the first routing-control set:

```powershell
.\scripts\install_claude_skills.ps1 -Names show-routing,route-activate
```

If you want the primary delegation flow too:

```powershell
.\scripts\install_claude_skills.ps1 -Names show-routing,route-activate,delegate
```

## Shared helper dependency

These skills assume the helper exists at:

```text
scripts/octoswitch_routing.py
```

## Base URL

Default:

```text
http://127.0.0.1:8787
```

Override if needed:

```powershell
$env:OCTOSWITCH_BASE_URL = "http://127.0.0.1:8787"
```

## Live routing-control endpoints used by the helper

- `GET /healthz`
- `GET /v1/routing/status`
- `GET /v1/routing/groups/:alias/members`
- `POST /v1/routing/groups/:alias/active-member`

## Skill status

Executable now:

- `show-routing`
- `route-activate`
- `delegate`

Design-stage only:

- `task-route`
