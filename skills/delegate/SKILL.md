---
name: delegate
description: Main delegation entrypoint. Analyze, split, and dispatch tasks to routed subagents, then summarize results.
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
- `/delegate --to <group> <task>`

Related command:

- `/task-route`: stores task-type routing preferences

## Route resolution

### Default route

```text
/delegate <task>
```

1. Analyze the task internally — identify distinct subtasks and their dependencies
2. For each subtask, classify its task kind and look up the matching task-route preference
3. Choose the execution strategy (see below)
4. **Immediately dispatch** the subagents — do NOT ask for confirmation

### Explicit route target

```text
/delegate --to <group> <task>
```

Resolve target as the specified group directly. The group name is the routing target — agents use it as their `model` field so requests go through the OctoSwitch gateway, where the active member can be switched in real time.

## Execution strategies

### Parallel Multi-Agent (independent subtasks)

**Use when**: The user asks for two or more independent things (different task kinds, different domains, "do X and Y").

**Critical rule**: When the request contains multiple independent subtasks, you MUST split them and dispatch to their respective agents in parallel using multiple Task tool calls in the same message. Do NOT merge unrelated subtasks into a single agent.

Examples:
- "研究石头为什么是圆的 + 讲个笑话" → research agent + joke agent, launched together
- "审查风险 + 搜索历史 bug" → review agent + search agent, launched together

### Serial Multi-Agent (dependent subtasks)

**Use when**: Subtask B needs subtask A's output to proceed.

Examples:
- "先搜索相关代码，然后重构" → search agent first, then implementation agent with search results

### Serial Single-Agent (single task)

**Use when**: The task is focused and can be handled by one agent.

Examples:
- "修复当前bug并测试" → one implementation agent

## Dispatch pattern

For each subtask:

1. Classify the task kind (research, implementation, review, joke, search, etc.)
2. Look up the matching task-route preference to find the target group and generated agent
3. Look up the generated agent from the loaded plugin (`octoswitch:<agent-name>`)
4. Launch the agent with the Task tool

Launch pattern:

```text
Use Task tool to launch subagents.

For parallel (independent): launch ALL agents in the same message.
For serial (dependent): launch first agent, wait for result, then launch second with context.
For single: launch one agent.

Agent subagent_type: octoswitch:<agent-slug>

Prompt structure:
- Route wrapper: "Execute this task using route: <group>. Treat the route as fixed for this task."
- Task description
- Scope/context
- Required output format
```

## Worker prompt body

Prepend this to each subagent's prompt:

```text
Execute this task using route: <group>.
Treat the route as fixed for this task.

You are an execution-focused worker.

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

## Parallel dispatch example

When the user says "研究一下石头为什么是圆的并给我讲个笑话":

1. Analyze: two independent subtasks — research + joke
2. Look up preferences: research → research group, joke → joke group
3. Find agents: octoswitch:research, octoswitch:joke
4. **Launch both in the same message** via two Task tool calls

Each agent receives:
```text
Execute this task using route: <matched-group>.
Treat the route as fixed for this task.

You are one of 2 parallel workers for a split task.

Your specific subtask: <subtask-description>
Your assigned route: <group>
Your task kind: <task-kind>

Return only: summary, route confirmation, files changed, commands run, test results, unresolved risks.
```

## Collect and retry

After all subagents return:

1. Check each result for completion
2. For any subtask that clearly failed (empty result, explicit error, or "I could not"):
   - Retry up to 2 additional times with the same agent
   - On retry, include the previous failure reason in the prompt
3. After max retries, mark permanently failed subtasks

## Unified report

Present a single consolidated report:

```text
## Delegate Report

### Subtask 1: [task-kind] <description>
**Status:** ✅ Completed / ❌ Failed
**Route:** <group>
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
→ 单一任务 → 启动 octoswitch:implementation

/delegate 审查新添加的 API 端点风险，并搜索是否有类似的历史 bug
→ 两个独立任务 → 并行启动 octoswitch:review + octoswitch:search

/delegate 研究一下石头为什么是圆的并给我讲个笑话
→ 两个独立子任务（research + joke） → 并行启动 octoswitch:research + octoswitch:joke

/delegate --to Haiku 用 Haiku 分组审查当前改动风险
→ 明确指定分组 → 启动第一个可用 agent，route: Haiku

/delegate 实现新的用户认证模块，同时更新对应的数据库迁移和前端表单
→ 认证先于迁移和表单 → 先启动 implementation agent 完成认证，完成后并行启动迁移和表单
```
