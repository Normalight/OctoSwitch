---
name: delegate
description: Main delegation entrypoint. Analyze, split, and dispatch tasks to routed subagents in parallel, then summarize results.
allowed-tools: ["Task", "Read"]
argument-hint: "<task description>"
---

# /delegate

Use this as the main execution command when work should be handed to fresh subagents.

This command must create one or more fresh subagents via the Task tool.
Do not execute the delegated work in the current session unless subagents are unavailable.

Compatibility forms:

- `/delegate ...`
- `/octoswitch:delegate ...`

When exported as a plugin artifact, publish this command under the `octoswitch` namespace.

## Command model

- `/delegate <task>`
- `/delegate --to <group>|<group/member> <task>`
- `/delegate --model <member> <task>`
- `/delegate --auto <task>`

Related command:

- `/task-route`: stores task-type routing preferences

## Route resolution

### Default route

```text
/delegate <task>
```

Resolve target as the group configured for the classified task kind (or `Sonnet` as fallback).

### Explicit Sonnet member

```text
/delegate --model <member> <task>
```

Resolve target as `Sonnet/<member>`.

### Explicit route target

```text
/delegate --to <group>|<group/member> <task>
```

Resolve target exactly as provided.

### Automatic routing mode

```text
/delegate --auto <task>
```

This mode should:

1. classify the task
2. consult `/task-route` preferences if available
3. choose the matching route and preferred generated subagent
4. launch the matching subagent

## Runtime behavior — Single task

When the task is a single, focused unit of work:

1. parse the route
2. gather minimal context
3. choose the worker agent
4. launch the worker with the Task tool
5. wait for the result
6. summarize the worker output for the user

## Runtime behavior — Multi-task splitting

When the task description contains multiple distinct subtasks (multiple verbs, multiple domains, or explicit multi-part requests):

### Phase 1: Analyze and Split

1. Read the task description
2. Identify distinct subtasks based on:
   - Explicit multiple requests ("do X, then Y", "A and B")
   - Different technical domains (frontend vs backend, code vs docs, etc.)
   - Different task kinds (implementation + review, search + implement, etc.)
3. For each subtask, determine the appropriate agent by:
   - If the user explicitly mentions a preference (e.g., "use review for this part"), use that task-route
   - Otherwise, classify the subtask kind and look up the matching `/task-route` preference
   - If no preference matches, use the first available generated agent
4. Cap at **3 subtasks** — merge related items if more are identified

### Phase 2: User Confirmation

Present the split plan to the user in this format:

```text
I'll split this into N subtasks:

1. [task-kind] <brief description> → agent: <agent-name>, route: <group>/<member>
2. [task-kind] <brief description> → agent: <agent-name>, route: <group>/<member>
3. [task-kind] <brief description> → agent: <agent-name>, route: <group>/<member>

Proceed?
```

Wait for user confirmation before launching. If the user objects, adjust and re-present.

### Phase 3: Parallel Dispatch

Launch ALL confirmed subagents in the same message using multiple Task tool calls. Each subagent receives:

```text
You are one of N parallel workers for a split task.

Your specific subtask: <subtask-description>
Your assigned route: <resolved-target>
Your task kind: <task-kind>
Treat the route and task kind as fixed for this task.

Return only these sections:
- route confirmation
- summary
- files changed
- commands run
- test results
- unresolved risks
```

The controller prepends the route wrapper:

```text
Execute this task using route: <resolved-target>.
Treat the route as fixed for this task.
```

### Phase 4: Collect and Retry

After all subagents return:

1. Check each result for completion
2. For any subtask that clearly failed (empty result, explicit error, or "I could not"):
   - Retry up to 2 additional times with the same agent
   - On retry, include the previous failure reason in the prompt
3. After max retries, mark permanently failed subtasks

### Phase 5: Unified Report

Present a single consolidated report:

```text
## Delegate Report

### Subtask 1: [task-kind] <description>
**Status:** ✅ Completed / ❌ Failed
**Route:** <group>/<member>
**Agent:** <agent-name>

<summary from worker>

---

### Subtask 2: [task-kind] <description>
...

---

## Summary
<overall summary combining all completed subtasks>

## Unresolved Risks
<consolidated risks from all subtasks>
```

The controller must not perform the delegated implementation itself.

## Worker selection

### Explicit direct routing

For `/delegate`, `/delegate --to`, and `/delegate --model`:

- look up generated agents from the loaded plugin (`octoswitch:<agent-name>`)
- pick the first available generated agent and pass the resolved OctoSwitch route as fixed task metadata
- if no generated agents are loaded, stop and tell the user to configure task-route preferences on the Skills page and sync

### Automatic routing

For `/delegate --auto`:

1. determine the task kind
2. read the local plugin config if available
3. look up the matching task route entry
4. if that route entry provides a generated delegate agent name, launch that exact agent
5. otherwise fall back to the first available generated agent, or report that no agents are configured

Generated agents are created from the OctoSwitch `Skills` page preferences.
After preferences change, the user must sync the local plugin and then run `/agents` to reload agents or restart the session.

Plugin-provided agents are typically addressed as `<plugin-name>:<agent-name>`, so do not drop the `octoswitch:` namespace when dispatching.

## Required Task launch pattern

Preferred controller behavior:

```text
Use Task tool to launch:
- explicit route mode: `octoswitch:<first_generated_agent>`
- auto mode with generated match: `octoswitch:<delegate_agent_name>`
- auto mode fallback: `octoswitch:<any_generated_agent>`

Include:
- route: <resolved-target>
- task kind: <classified-task-kind-or-explicit>
- task: <delegated-task>
- scope/context: <minimal necessary context>
- required output: route confirmation / summary / files changed / commands run / test results / unresolved risks
```

## Required result format

Each delegated subagent should return only:

- route confirmation
- summary
- files changed
- commands run
- test results
- unresolved risks

## Recommended worker prompt body

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
- route confirmation
- summary
- files changed
- commands run
- test results
- unresolved risks
```

## Route wrapper

Prepend a short wrapper:

```text
Execute this task using route: <resolved-target>.
Treat the route as fixed for this task.
```

## Failure handling

If the resolved target is `Sonnet` but the project does not define a `Sonnet` group in OctoSwitch:

- stop
- explain that `Sonnet` is missing
- suggest creating a `Sonnet` group or using `/delegate --to <existing-group>/<member> ...`

If the user supplies `--model <member>` but that member does not exist under `Sonnet`, report the routing error directly.

If `/delegate --auto` resolves to a generated agent name that is not currently loaded:

- stop
- explain that the local plugin agents are stale
- tell the user to sync the local OctoSwitch plugin, then run `/agents` or restart the session

If no generated agents are registered:

- stop
- explain that at least one task-route preference must be configured
- direct the user to the Skills page to add preferences and sync

If the platform does not support subagents or the Task tool is unavailable:

- stop
- explain that `/delegate` requires subagent support
- do not silently fall back to doing the work in the current session

## Practical examples

```text
/delegate 按当前确认方案完成实现并测试
/delegate 审查新添加的 API 端点风险，并搜索是否有类似的历史 bug
/delegate --model gpt-5.4 修复当前 bug 并汇报测试结果
/delegate --to Sonnet/gpt-5.4 审查当前改动风险
/delegate --auto 分析当前 bug，完成修复并做回归检查
/delegate 实现新的用户认证模块，同时更新对应的数据库迁移和前端表单
```
