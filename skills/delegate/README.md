# `/delegate` usage

Primary delegation command for this project.

## Supported forms

```text
/delegate <task>
/delegate --model <member> <task>
/delegate --to <group>|<group/member> <task>
/delegate --auto <task>
```

Examples:

```text
/delegate 按当前确认方案完成实现并测试
/delegate --model gpt-5.4 修复当前 bug 并汇报测试结果
/delegate --to Sonnet/gpt-5.4 审查当前改动风险
/delegate --auto 搜索相关代码入口并汇总影响范围
```

## Resolution rules

- `/delegate <task>` -> `Sonnet`
- `/delegate --model <member> <task>` -> `Sonnet/<member>`
- `/delegate --to <group>|<group/member> <task>` -> explicit target exactly as provided
- `/delegate --auto <task>` -> classify task, then use configured task-route preference when available

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
Execute this task using route: Sonnet/gpt-5.4.
Treat the route as fixed for this task.
```

## Notes

- This command assumes the project has a `Sonnet` routing group when no explicit target is provided.
- If the project does not have `Sonnet`, prefer `/delegate --to <group> ...`.
- Direct routing uses the fallback worker `octoswitch:octoswitch-delegate-default-worker`.
- `/delegate --auto` can launch a generated preference-specific worker agent after the local plugin is synced and reloaded with `/agents`.
