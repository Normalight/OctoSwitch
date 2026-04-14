# `/delegate` usage

Main delegation command for this project — a composite skill with plan-first execution, two-stage review gates, and structured status protocol.

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

- `/delegate <task>` — controller decomposes the request into a structured task plan with dependencies, classifies each task as parallel/serial/standalone, builds an execution schedule in waves, and dispatches agents accordingly
- `/delegate --to <group> <task>` — explicit group target, all tasks resolve to the specified group

## Execution phases

### Phase 1: Plan (task decomposition)

The controller builds an explicit task plan before any agent is launched:

1. **Decompose** — break into focused, scoped tasks with task kinds
2. **Classify** — mark each task as `parallel`, `serial`, or `standalone`
3. **Map dependencies** — `blockedBy` / `blocks` relationships between tasks
4. **Define completion** — concrete `doneWhen` criteria per task
5. **Register tasks** — create TodoWrite entries for progress tracking
6. **Schedule waves** — Wave 1 has no blockers; Wave N depends on Wave N-1
7. **Resolve routing** — look up task-route preferences for target groups
8. **Validate** — check no circular deps, all references valid, at least one route

### Phase 2: Dispatch (execute waves with review gates)

- All tasks in a wave launch together (parallel Task tool calls)
- Wait for the full wave before starting the next
- Each worker receives structured context: task ID, description, scope, prior task outputs, completion criteria
- **Review gate for serial tasks**: verify `doneWhen` criteria before proceeding to dependent tasks
- **Stop-on-blocker**: if a task is `BLOCKED`, do not launch dependent tasks; escalate to user
- **Structured status**: workers report as `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, or `NEEDS_CONTEXT`

### Phase 3: Report (collect, verify, summarize)

- Progressively report each completed task
- Retry failed tasks up to 2 times
- **Verification gate**: re-check every task's `doneWhen` criteria before claiming completion
- Present a unified summary table when all tasks complete

## Structured status protocol

Workers classify their result as ONE of:

| Status | Meaning | Controller action |
|--------|---------|-------------------|
| `DONE` | All `doneWhen` criteria met, no issues | Proceed to dependent tasks |
| `DONE_WITH_CONCERNS` | Criteria met, but risks identified | Proceed; flag concerns in report |
| `BLOCKED` | Concrete blocker prevents progress | **STOP**; escalate to user |
| `NEEDS_CONTEXT` | Missing information needed | Retry once with context; escalate if still stuck |

## Two-stage review for serial tasks

When a **serial** task completes:

1. **Stage 1 — Spec compliance**: verify the worker's output actually meets the `doneWhen` criteria
2. **Stage 2 — Code quality**: if criteria met, scan for obvious issues (security, edge cases, broken references)
3. If both pass → proceed to dependent tasks
4. If Stage 1 fails → retry with specific feedback (max 2 retries)
5. If Stage 2 flags concerns → proceed but note in report

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

## Structured status protocol
Classify your result as: DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT

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
