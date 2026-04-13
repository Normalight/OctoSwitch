# `/task-route` design notes

Design-stage preference skill for route-aware automatic delegation.

## What it is for

Use `/task-route` to define recommended routing targets for task kinds such as:

- implementation
- review
- search
- debugging
- refactor

## Current status

OctoSwitch now includes a concrete MVP preference store for task-route settings.

Current UI flow:

- enable `Skills` in Settings
- open the `Skills` tab from the main navigation
- add or edit task-route preferences there

The Claude-side slash command still represents the same preference contract, while OctoSwitch now provides the persisted backing store.

## First-run guidance

If task-route preferences are not configured yet, recommend this minimal starting set:

```text
/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7
```

These are recommended defaults, not hardcoded mandatory values.

## What it affects

`/task-route` does not execute tasks by itself.

Its purpose is to give `/delegate --auto` a stable preference layer when it needs to choose:

- task route
- subagent role
- default target for a task kind

## Relationship to other commands

- `/show-routing` checks available routes
- `/task-route` stores route preferences
- `/delegate --auto` builds the automatic route plan from those preferences
- `/delegate` runs the chosen route
