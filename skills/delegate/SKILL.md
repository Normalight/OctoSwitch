---
name: delegate
description: Main delegation entrypoint. Resolve an OctoSwitch route, then launch a real subagent instead of doing the work in the controller session.
allowed-tools: ["Task", "Read"]
argument-hint: "[--auto] [--to <group>|<group/member>] [--model <member>] <task>"
---

# /delegate

Use this as the main execution command when work should be handed to a fresh subagent.

This command must create a fresh subagent via the Task tool.
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

Resolve target as `Sonnet`.

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

## Runtime behavior

The current session acts as controller only:

1. parse the route
2. gather minimal context
3. choose the worker agent
4. launch the worker with the Task tool
5. wait for the result
6. summarize the worker output for the user

The controller must not perform the delegated implementation or review itself.

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

The delegated subagent should return only:

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
/delegate --model gpt-5.4 修复当前 bug 并汇报测试结果
/delegate --to Sonnet/gpt-5.4 审查当前改动风险
/delegate --auto 分析当前 bug，完成修复并做回归检查
```
