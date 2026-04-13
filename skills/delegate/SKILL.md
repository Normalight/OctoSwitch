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

### Phase 1: Plan (structured task decomposition)

The controller analyzes the request and builds an **explicit task plan** before any agent is launched.
This plan is the controller's internal blueprint — do NOT present it to the user for approval.

#### Step 1: Decompose into tasks

Break the request into the smallest set of focused, independently scoped tasks.
Each task must have:

- `id` — sequential number (1, 2, 3...)
- `kind` — task kind for route lookup (`implementation`, `review`, `search`, `debug`, `refactor`, `test`, `docs`, or other user-defined kind)
- `description` — what this task does, in one sentence
- `scope` — specific files/areas/deliverables this task touches
- `mode` — execution mode (see below)
- `blocks` — list of task IDs that **cannot start** until this task completes
- `blockedBy` — list of task IDs that **must complete** before this task can start
- `doneWhen` — concrete completion criteria (what does "done" look like for this task?)

#### Step 2: Classify execution mode

For each task, assign one mode:

| Mode | Meaning | Launch strategy |
|------|---------|-----------------|
| `parallel` | No dependencies on other tasks | Launch in the same message as other parallel tasks |
| `serial` | Depends on one or more prior tasks | Launch after all `blockedBy` tasks complete |
| `standalone` | Single task, no other tasks in plan | Launch one agent, done |

#### Step 3: Resolve routing

For each task:
1. Classify the task `kind`
2. Look up the matching task-route preference for target `group`
3. If no preference matches, use the default group or the explicit `--to` group
4. Look up the generated agent slug: `octoswitch:<agent-name>`

#### Step 4: Build execution schedule

Group tasks into **waves**:

- **Wave 1**: All tasks with no `blockedBy` dependencies — launch together (parallel)
- **Wave 2**: All tasks whose `blockedBy` tasks are all in Wave 1 — launch together after Wave 1 completes
- **Wave N**: Continue until all tasks are scheduled

Example plan structure:

```text
Plan — [brief summary of the delegated request]

| # | kind           | mode     | blockedBy | blocks    | route  | agent                    |
|---|----------------|----------|-----------|-----------|--------|--------------------------|
| 1 | search         | parallel | —         | 2, 3      | Haiku  | octoswitch:search        |
| 2 | implementation | serial   | 1         | —         | Sonnet | octoswitch:implementation|
| 3 | review         | serial   | 1         | —         | Opus   | octoswitch:review        |
```

Execution order:
- Wave 1: Task 1 (search) — standalone launch
- Wave 2: Tasks 2 + 3 (implementation + review) — launch together after Task 1 completes, both receive Task 1's output

#### Step 5: Validate the plan

Before dispatching, verify:
- Every task has exactly one `mode`
- No circular dependencies (if task A blocks B, B must not block A)
- All `blockedBy` references point to valid task IDs
- Each task has a concrete `doneWhen` criterion
- At least one task-route preference matches, or an explicit `--to` group is provided

If validation fails, explain the error and stop. Do NOT dispatch.

### Phase 2: Dispatch (execute waves)

#### Launch pattern

For each wave:
- **Parallel wave** (2+ tasks): launch ALL agents in the same message via multiple Task tool calls
- **Single wave** (1 task): launch one agent
- Wait for ALL agents in the current wave to complete before starting the next wave

#### Worker prompt template

Each subagent receives a structured prompt with all necessary context:

```text
Execute this task using route: <group>.
Treat the route as fixed for this task.

You are Task #<id> of <total> in a split delegation.

## Your assigned work
<description>

## Scope
<scope>

## Context from prior tasks
<if blockedBy: include summarized output of all blocking tasks>
<if no blockedBy: "This is the first task — no prior context needed.">

## Done when
<doneWhen>

## You are an execution-focused worker

Do:
- execute the requested task within scope
- run relevant checks or tests when appropriate
- fix direct follow-up issues only when clearly in scope

Do not:
- change the routing target
- re-plan the whole task unless blocked
- broaden scope beyond what's described
- return long reasoning dumps

Return only:
- route confirmation (state your route: <group>)
- summary of what you did
- files changed
- commands run
- test results
- unresolved risks
```

### Phase 3: Report (collect, retry, summarize)

#### Progressive reporting

**As each wave completes, immediately report to the user** — do NOT wait for all waves.

Format for each completed task:
```text
✅ Task #<id> [<kind>] completed (route: <group>)

<worker summary>

Files changed: <list or "none">
```

#### Retry failed tasks

For any task that clearly failed (empty result, explicit error, or "I could not"):
1. Retry up to 2 additional times with the same agent
2. On retry, include the previous failure reason in the prompt:
   ```text
   Previous attempt failed because: <reason>. Please retry with this in mind.
   ```
3. After max retries, mark as permanently failed:
   ```text
   ❌ Task #<id> [<kind>] failed (route: <group>)
   <error or failure reason>
   ```

#### Final unified report

When ALL tasks have returned (including retries):

```text
## Delegate Report — Summary

| # | Task | Kind | Status | Route |
|---|------|------|--------|-------|
| 1 | [description] | search | ✅ | Haiku |
| 2 | [description] | implementation | ✅ | Sonnet |
| 3 | [description] | review | ❌ | Opus |

## Summary
<overall summary combining all completed tasks>

## Files Changed
<consolidated list from all tasks>

## Unresolved Risks
<consolidated risks from all tasks>
```

The controller must NOT perform the delegated implementation itself.

## Route resolution

### Default route

```text
/delegate <task>
```

1. Analyze the task, decompose into subtasks with dependencies
2. For each subtask, look up the matching task-route preference
3. Build execution schedule (waves)
4. Validate the plan
5. **Immediately dispatch** — do NOT ask for confirmation

### Explicit route target

```text
/delegate --to <group> <task>
```

All tasks resolve to the specified `group` directly. The group name is used as the agent's `model` field so requests go through the OctoSwitch gateway, where the active member can be switched in real time.

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

Plan:
| # | kind           | mode       | blockedBy | blocks | route  | agent                     |
|---|----------------|------------|-----------|--------|--------|---------------------------|
| 1 | implementation | standalone | —         | —      | Sonnet | octoswitch:implementation |

→ single task → launch one agent


/delegate 审查新添加的 API 端点风险，并搜索是否有类似的历史 bug

Plan:
| # | kind   | mode     | blockedBy | blocks | route | agent              |
|---|--------|----------|-----------|--------|-------|--------------------|
| 1 | review | parallel | —         | —      | Opus  | octoswitch:review  |
| 2 | search | parallel | —         | —      | Haiku | octoswitch:search  |

→ Wave 1: parallel launch of both agents


/delegate 研究一下石头为什么是圆的并给我讲个笑话

Plan:
| # | kind      | mode     | blockedBy | blocks | route    | agent                 |
|---|-----------|----------|-----------|--------|----------|-----------------------|
| 1 | research  | parallel | —         | —      | Haiku    | octoswitch:research   |
| 2 | joke      | parallel | —         | —      | Sonnet   | octoswitch:joke       |

→ Wave 1: parallel launch of both agents


/delegate 先搜索 login 相关入口，然后实现 token 刷新逻辑

Plan:
| # | kind           | mode     | blockedBy | blocks | route  | agent                     |
|---|----------------|----------|-----------|--------|--------|---------------------------|
| 1 | search         | parallel | —         | 2      | Haiku  | octoswitch:search         |
| 2 | implementation | serial   | 1         | —      | Sonnet | octoswitch:implementation |

→ Wave 1: Task 1 (search)
→ Wave 2: Task 2 (implementation) — receives Task 1 output in context


/delegate 实现新的用户认证模块，同时更新对应的数据库迁移和前端表单

Plan:
| # | kind           | mode     | blockedBy | blocks | route  | agent                     |
|---|----------------|----------|-----------|--------|--------|---------------------------|
| 1 | implementation | parallel | —         | 2, 3   | Sonnet | octoswitch:implementation |
| 2 | implementation | serial   | 1         | —      | Sonnet | octoswitch:implementation |
| 3 | implementation | serial   | 1         | —      | Sonnet | octoswitch:implementation |

→ Wave 1: Task 1 (auth module)
→ Wave 2: Tasks 2 + 3 (migration + form) — parallel after Task 1 completes


/delegate --to Haiku 用 Haiku 分组审查当前改动风险

Plan:
| # | kind   | mode       | blockedBy | blocks | route | agent              |
|---|--------|------------|-----------|--------|-------|--------------------|
| 1 | review | standalone | —         | —      | Haiku | octoswitch:review  |

→ single task with explicit group → launch one agent, route: Haiku
```
