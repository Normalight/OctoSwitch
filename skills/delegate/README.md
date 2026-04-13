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
/delegate --to Haiku 用 Haiku 分组搜索相关代码
```

## Resolution rules

- `/delegate <task>` — main model creates a structured plan (subtasks, dependencies, task kinds), classifies each subtask, dispatches to respective agents in parallel or serial
- `/delegate --to <group> <task>` — explicit group target, agents use group name as model field for OctoSwitch gateway routing

## Execution phases

1. **Plan** — the main model creates an internal blueprint: subtasks, dependencies, task kinds, target groups
2. **Dispatch** — based on dependency analysis, launches agents in parallel (independent) or serial (dependent)
3. **Report** — collects results, retries failures, presents unified summary

## Recommended worker prompt

```text
You are an execution-focused worker.

Your assigned route is fixed for this task.
Use the current thread context and complete the delegated work within scope.

Do:
- execute the approved task
- stay within the requested scope
- run relevant checks or tests when appropriate
- fix direct follow-up issues only when they are clearly in scope

Do not:
- change the routing target
- re-plan the whole task unless blocked
- broaden scope on your own
- return long reasoning dumps

Return only:
- summary
- files changed
- commands run
- test results
- unresolved risks
```

## Route wrapper example

```text
Execute this task using route: Sonnet.
Treat the route as fixed for this task.
```

## Notes

- Direct routing uses generated agents from the OctoSwitch Skills page preferences.
- If no generated agents are registered, configure task-route preferences on the Skills page and sync.
