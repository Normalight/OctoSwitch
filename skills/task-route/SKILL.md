---
name: task-route
description: Define a preferred OctoSwitch routing target for a task kind.
allowed-tools: Read
---

# /task-route

Use this skill when the user wants to define a preferred target route for a task type.

## Goal

Store a recommendation that future automatic delegation can use when choosing routes.

This is a preference-setting command, not an execution command.

## Simplified command model

To keep the delegation UX small and understandable:

- `/delegate` = execute delegated work
- `/delegate --auto` = execute with automatic route selection
- `/task-route` = store route preferences used by automatic delegation

This means `/task-route` should be described as configuration for `/delegate --auto`, not as a competing execution command.

## Supported forms

```text
/task-route <task-kind> --target <group>/<member>
/task-route <task-kind> --group <group>
/task-route <task-kind> --model <member>
```

Examples:

```text
/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7
```

## Intended semantics

These rules are recommendations, not hard overrides.

Suggested priority:

1. explicit target in the current task
2. task-route preference
3. default active member of the target group

## Recommended task kinds

Start with a small, stable set:

- `implementation`
- `review`
- `search`
- `debugging`
- `refactor`

## Recommended initial mapping

```text
implementation -> Sonnet/gpt-5.4
review -> Opus/gpt-5.4
search -> Haiku/MiniMax-M2.7
```

## Current status

OctoSwitch now has a concrete MVP implementation for task-route preferences.

Current implementation surface:

- Settings -> enable `Skills`
- Main UI -> `Skills` tab
- persisted task-route preference records
- add / edit / delete task-route entries
- optional prompt template per task kind

At the Claude Code slash-command layer, this skill acts as the routing preference contract for `/delegate --auto`. The storage and editing side now exists in OctoSwitch.

## First-run onboarding

If this looks like the first time task-route preferences are being configured, recommend a minimal starting set instead of forcing the user to design the whole route table up front.

Recommended onboarding message:

```text
This looks like the first time task-route preferences are being configured.
A minimal recommended starting set is:

/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7

After that, /delegate --auto can use those preferences when choosing routes and subagents.
```

### Onboarding behavior

If no task-route preferences exist yet:

1. explain that a small initial mapping is enough to get started
2. recommend the minimal default mapping
3. present the mapping as editable preferences, not fixed system rules

### Important rule

These initial values are recommended defaults, not mandatory hardcoded values.

If the user already has a preferred route layout, that preference should win.

## First configuration guidance

If the user has not configured any task-route preferences yet, recommend this minimal starting set:

```text
/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7
```

Present these as recommended defaults, not mandatory values.

## Recommended explanation to users

When explaining the command set, prefer this wording:

```text
/delegate handles execution.
/task-route defines default route preferences used by /delegate --auto.
```

Avoid presenting `/task-route` as a third execution entrypoint.
