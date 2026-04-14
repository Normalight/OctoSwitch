---
name: delegate
description: Main delegation entrypoint. Plan, split, and dispatch tasks to routed subagents with two-stage review gates and verification-before-completion discipline.
allowed-tools: ["Task", "Read", "TodoWrite"]
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

## Architecture: Controller-Subagent Separation

The controller (this session) orchestrates. Subagents execute with fresh context.
The controller never performs the delegated work itself.

### Key principles

1. **Fresh context per task** — each subagent gets a clean context, not the full conversation history
2. **Two-stage review** — serial tasks pass through a review gate before the next task starts
3. **Structured status** — workers report using a fixed protocol: `DONE`, `DONE_WITH_CONCERNS`, `BLOCKED`, `NEEDS_CONTEXT`
4. **Verification gate** — before marking any task complete, verify the `doneWhen` criteria are actually met
5. **Stop-on-blocker** — if a task is `BLOCKED`, do not proceed to dependent tasks; escalate to user
6. **Model selection by complexity** — use task-route preferences; if none exist, route by complexity heuristic

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
| `serial` | Depends on one or more prior tasks | Launch after all `blockedBy` tasks complete; review gate applies |
| `standalone` | Single task, no other tasks in plan | Launch one agent, done |

#### Step 3: Resolve routing

For each task:
1. Classify the task `kind`
2. Look up the matching task-route preference for target `group`
3. If no preference matches, use the default group or the explicit `--to` group
4. Look up the generated agent slug: `octoswitch:<agent-name>`

#### Step 4: Register tasks in TodoWrite

Create a TodoWrite entry for each task to track progress:

```
TodoCreate({
  subject: "Task #<id> [<kind>]: <description>",
  description: "Route: <group> | Done when: <doneWhen>",
  status: "pending"
})
```

Update status to `in_progress` when launching, `completed` when verified done.

#### Step 5: Build execution schedule

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

#### Step 6: Validate the plan

Before dispatching, verify:
- Every task has exactly one `mode`
- No circular dependencies (if task A blocks B, B must not block A)
- All `blockedBy` references point to valid task IDs
- Each task has a concrete `doneWhen` criterion
- At least one task-route preference matches, or an explicit `--to` group is provided

If validation fails, explain the error and stop. Do NOT dispatch.

### Phase 2: Dispatch (execute waves with review gates)

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

## Structured status protocol

Before finishing, classify your result as ONE of:

- **DONE**: All `doneWhen` criteria are met. No unresolved issues.
- **DONE_WITH_CONCERNS**: Criteria met, but you identified risks or follow-ups worth flagging.
- **BLOCKED**: Cannot proceed due to a concrete blocker (describe what and why).
- **NEEDS_CONTEXT**: Missing information needed to proceed (specify what).

**Do NOT** claim DONE if you did not verify the criteria.

## You are an execution-focused worker

Do:
- execute the requested work within scope
- run relevant checks or tests when appropriate
- fix direct follow-up issues only when clearly in scope

Do not:
- change the routing target
- re-plan the whole task unless blocked
- broaden scope beyond what's described
- return long reasoning dumps

Return only:
- route confirmation (state your route: <group>)
- status: DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
- summary of what you did (3-5 bullets max)
- files changed (paths only)
- commands run
- test results (pass/fail counts)
- unresolved risks (if any)
```

#### Review gate for serial tasks

When a **serial** task completes:

1. **Verify** the `doneWhen` criteria are actually met by examining the worker's output
2. If `DONE` or `DONE_WITH_CONCERNS` and criteria verified → proceed to dependent tasks
3. If `BLOCKED` → **STOP**. Do not launch dependent tasks. Escalate to user with blocker details.
4. If `NEEDS_CONTEXT` → retry once with additional context from the controller. If still stuck, escalate.
5. If criteria NOT verified → retry the task with specific feedback on what was missing. Max 2 retries.

#### Parallel task handling

Parallel tasks have no review gates between them — they launch together and report independently.
Each still gets verified against its own `doneWhen` criteria.

### Phase 3: Report (collect, verify, summarize)

#### Progressive reporting

**As each wave completes, immediately report to the user** — do NOT wait for all waves.

Format for each completed task:
```text
✅ Task #<id> [<kind>] completed (route: <group>) — STATUS: <DONE|DONE_WITH_CONCERNS>

<worker summary>

Files changed: <list or "none">
```

#### Retry logic

For any task that clearly failed:
1. Retry up to 2 additional times with the same agent
2. On retry, include the previous failure reason:
   ```text
   Previous attempt failed because: <reason>. Please retry with this in mind.
   ```
3. After max retries, mark as permanently failed:
   ```text
   ❌ Task #<id> [<kind>] failed (route: <group>)
   <error or failure reason>
   ```

#### Verification before completion

Before presenting the final summary:
1. Re-check every task's `doneWhen` criteria against the worker's output
2. If any task's criteria are not met, flag it even if the worker claimed DONE
3. Do not claim "all tasks complete" unless every `doneWhen` is verified

#### Final unified report

When ALL tasks have returned (including retries):

```text
## Delegate Report — Summary

| # | Task | Kind | Status | Route |
|---|------|------|--------|-------|
| 1 | [description] | search | ✅ | Haiku |
| 2 | [description] | implementation | ⚠️ concerns | Sonnet |
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
5. Register tasks in TodoWrite
6. **Immediately dispatch** — do NOT ask for confirmation

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

→ single task → launch one agent → verify doneWhen → report


/delegate 审查新添加的 API 端点风险，并搜索是否有类似的历史 bug

Plan:
| # | kind   | mode     | blockedBy | blocks | route | agent              |
|---|--------|----------|-----------|--------|-------|--------------------|
| 1 | review | parallel | —         | —      | Opus  | octoswitch:review  |
| 2 | search | parallel | —         | —      | Haiku | octoswitch:search  |

→ Wave 1: parallel launch of both agents → report each → summarize


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
→ verify search results are complete
→ Wave 2: Task 2 (implementation) — receives Task 1 output in context
→ review gate: verify implementation matches search findings


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
