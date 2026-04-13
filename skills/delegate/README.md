# `/delegate` usage

Primary delegation command for this project.

## Supported forms

```text
/delegate <task>
/delegate --to <group> <task>
```

Examples:

```text
/delegate 按当前确认方案完成实现并测试
/delegate --to Haiku 用 Haiku 分组审查当前改动风险
```

## Resolution rules

- `/delegate <task>` — main model decomposes the request into a structured task plan with dependencies, classifies each task as parallel/serial/standalone, builds an execution schedule in waves, and dispatches agents accordingly
- `/delegate --to <group> <task>` — explicit group target, all tasks resolve to the specified group

## Execution phases

### Phase 1: Plan (task decomposition)

The main model builds an explicit task plan before any agent is launched:

1. **Decompose** — break into focused, scoped tasks with task kinds
2. **Classify** — mark each task as `parallel`, `serial`, or `standalone`
3. **Map dependencies** — `blockedBy` / `blocks` relationships between tasks
4. **Define completion** — concrete `doneWhen` criteria per task
5. **Schedule waves** — Wave 1 has no blockers; Wave N depends on Wave N-1
6. **Resolve routing** — look up task-route preferences for target groups
7. **Validate** — check no circular deps, all references valid, at least one route

### Phase 2: Dispatch (execute waves)

- All tasks in a wave launch together (parallel Task tool calls)
- Wait for the full wave before starting the next
- Each worker receives structured context: task ID, description, scope, prior task outputs, completion criteria

### Phase 3: Report (collect, retry, summarize)

- Progressively report each completed task
- Retry failed tasks up to 2 times
- Present a unified summary table when all tasks complete

## Recommended worker prompt

Each subagent receives:

```text
Execute this task using route: <group>.
Treat the route as fixed for this task.

You are Task #<id> of <total> in a split delegation.

## Your assigned work
<description>

## Scope
<scope>

## Context from prior tasks
<prior task outputs, or "no prior context needed">

## Done when
<completion criteria>

## You are an execution-focused worker
...
```

## Route wrapper example

```text
Execute this task using route: Sonnet.
Treat the route as fixed for this task.
```

## Notes

- Direct routing uses generated agents from the OctoSwitch Skills page preferences.
- If no generated agents are registered, configure task-route preferences on the Skills page and sync.
