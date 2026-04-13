---
name: delegate
description: Main delegation entrypoint. Plan, split, and dispatch tasks to routed subagents, then summarize results.
allowed-tools: ["Task", "Read"]
argument-hint: "<task description>"
---

# /delegate

Use this as the main execution command when work should be handed to fresh subagents.

This command must create one or more fresh subagents via the **Task tool**.
Do NOT use the Skill tool. The agents are launched via the Task tool's `subagent_type` parameter.
Do not execute the delegated work in the current session unless subagents are unavailable.

Compatibility forms:

- `/delegate ...`
- `/octoswitch:delegate ...`

When exported as a plugin artifact, publish this command under the `octoswitch` namespace.

## Command model

- `/delegate <task>`
- `/delegate --to <group> <task>`

Related command:

- `/task-route`: stores task-type routing preferences

## Plan-first execution flow

Every `/delegate` follows three phases:

### Phase 1: Plan (this session)

The controller (this session) analyzes the request and creates a structured plan.
The plan must identify:

1. **Distinct subtasks** — each with a clear responsibility
2. **Dependencies** — which subtasks depend on others' output
3. **Task kinds** — classification for route lookup (implementation, review, search, etc.)
4. **Target groups** — from task-route preferences or explicit `--to` flag

The plan is NOT presented to the user for approval (too slow for delegation).
It is the controller's internal blueprint for dispatch.

### Phase 2: Dispatch (this session → subagents)

Based on dependency analysis, choose one execution strategy:

#### Parallel Multi-Agent (independent subtasks)

**Use when**: Subtasks have no dependencies on each other.

**Critical rule**: When the request contains multiple independent subtasks, you MUST split them and dispatch to their respective agents in parallel using multiple Task tool calls in the same message. Do NOT merge unrelated subtasks into a single agent.

Launch ALL independent agents in one message — they execute concurrently.

#### Serial Multi-Agent (dependent subtasks)

**Use when**: Subtask B needs Subtask A's output.

Dispatch sequentially: launch A, wait for result, include A's output in B's prompt.

#### Serial Single-Agent (single focused task)

**Use when**: One task kind covers the entire request.

Launch one agent.

### Phase 3: Report (this session ← subagents)

Collect results, retry failures, present final summary.

## Route resolution

### Default route

```text
/delegate <task>
```

1. Analyze the task, create a structured plan (subtasks, dependencies, task kinds)
2. For each subtask, look up the matching task-route preference
3. Choose the execution strategy based on dependencies
4. **Immediately dispatch** — do NOT ask for confirmation

### Explicit route target

```text
/delegate --to <group> <task>
```

Resolve target as the specified group directly. The group name is the routing target — agents use it as their `model` field so requests go through the OctoSwitch gateway, where the active member can be switched in real time.

## Dispatch pattern

For each subtask:

1. Classify the task kind (implementation, review, search, debug, refactor, etc.)
2. Look up the matching task-route preference to find the target group
3. Look up the generated agent from the loaded plugin (`octoswitch:<agent-name>`)
4. Launch the agent with the **Task tool** — NOT the Skill tool

**Important**: The available agents are `octoswitch:<agent-slug>` (e.g. `octoswitch:implementation`, `octoswitch:review`, `octoswitch:search`). These are **agent types** loaded by the plugin, NOT skill names. Use the Task tool's `subagent_type` parameter to launch them.

Launch pattern:

```text
Use the Task tool (NOT Skill tool) to launch subagents.

For parallel (independent): launch ALL agents in the same message.
For serial (dependent): launch first agent, wait for result, then launch second with context.
For single: launch one agent.

Task tool call:
- subagent_type: "octoswitch:<agent-slug>"  (e.g. "octoswitch:implementation")
- prompt: (see below)
```

## Worker prompt body

Prepend this to each subagent's prompt:

```text
Execute this task using route: <group>.
Treat the route as fixed for this task.

You are an execution-focused worker.

Do:
- execute the requested task
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

## Parallel dispatch example

When the user says "研究一下石头为什么是圆的并给我讲个笑话":

1. Analyze: two independent subtasks — research + joke
2. Look up preferences: research → research group, joke → joke group
3. Find agents: octoswitch:research, octoswitch:joke
4. **Launch both in the same message** via two Task tool calls

Each agent receives its prompt via the Task tool:
```text
Execute this task using route: <matched-group>.
Treat the route as fixed for this task.

You are one of 2 parallel workers for a split task.

Your specific subtask: <subtask-description>
Your assigned route: <group>
Your task kind: <task-kind>

Return only: summary, route confirmation, files changed, commands run, test results, unresolved risks.
```

## Serial dispatch example

When the user says "先搜索相关代码，然后总结影响范围":

1. Analyze: search must complete before summary can use its results
2. Launch search agent first: `octoswitch:search`
3. Wait for search results
4. Launch summary agent with search output in prompt: `octoswitch:implementation` (or appropriate)

## Collect, report progressively, and retry

When running parallel agents:

1. **As each subagent completes, immediately report its result to the user** — do NOT wait for all to finish.
   Format:
   ```text
   ✅ [task-kind] <description> completed (route: <group>)

   <summary from worker>
   ```
2. For any subtask that clearly failed (empty result, explicit error, or "I could not"):
   - Retry up to 2 additional times with the same agent
   - On retry, include the previous failure reason in the prompt
3. After max retries, mark permanently failed and report:
   ```text
   ❌ [task-kind] <description> failed (route: <group>)
   <error or failure reason>
   ```

When **all** subagents have returned (including retries):

1. Present a final unified report:

```text
## Delegate Report — Summary

| # | Task | Status | Route |
|---|------|--------|-------|
| 1 | [task-kind] <description> | ✅ / ❌ | <group> |
| 2 | [task-kind] <description> | ✅ / ❌ | <group> |

## Summary
<overall summary combining all completed subtasks>

## Unresolved Risks
<consolidated risk from all subtasks>
```

The controller must not perform the delegated implementation itself.

## Generated agents

Generated agents are created from the OctoSwitch `Skills` page preferences.
After preferences change, the user must sync the local plugin and then run `/agents` to reload agents or restart the session.

Plugin-provided agents are addressed as `octoswitch:<agent-name>` — do not drop the namespace.

If no generated agents are registered:

- stop
- explain that at least one task-route preference must be configured
- direct the user to the Skills page to add preferences and sync

## Failure handling

If the resolved target group does not exist in the generated agents:

- stop
- explain the routing error
- suggest creating the missing group or using `/delegate --to <existing-group> ...`

If the platform does not support subagents or the Task tool is unavailable:

- stop
- explain that `/delegate` requires subagent support
- do not silently fall back to doing the work in the current session

## Practical examples

```text
/delegate 按当前确认方案完成实现并测试
→ single task → launch octoswitch:implementation

/delegate 审查新添加的 API 端点风险，并搜索是否有类似的历史 bug
→ two independent tasks → parallel: octoswitch:review + octoswitch:search

/delegate 研究一下石头为什么是圆的并给我讲个笑话
→ two independent subtasks (research + joke) → parallel: octoswitch:research + octoswitch:joke

/delegate --to Haiku 用 Haiku 分组审查当前改动风险
→ explicit group target → launch first available agent, route: Haiku

/delegate 实现新的用户认证模块，同时更新对应的数据库迁移和前端表单
→ auth first, then migration and form are dependent → serial: implementation(agent 1), then parallel: migration(agent 2) + form(agent 3)

/delegate 先搜索 login 相关入口，然后实现 token 刷新逻辑
→ search must complete before implementation → serial: search, then implementation with search results
```
