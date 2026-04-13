---
name: delegate
description: Main delegation entrypoint. Execute directly, route explicitly, or let routing choose automatically.
allowed-tools: Read
argument-hint: "[--auto] [--to <group>|<group/member>] [--model <member>] <task>"
---

# /delegate

Use this as the single main execution command when the user wants Claude Code to hand work to a routed subagent flow.

Compatibility forms:

- `/delegate ...`
- `/octoswitch:delegate ...`

When exported as a plugin artifact, publish this command under the `octoswitch` namespace.

## Command model

Recommended command surface:

- `/delegate <task>`
- `/octoswitch:delegate <task>`
- `/delegate --to <group>|<group/member> <task>`
- `/octoswitch:delegate --to <group>|<group/member> <task>`
- `/delegate --model <member> <task>`
- `/octoswitch:delegate --model <member> <task>`
- `/delegate --auto <task>`
- `/octoswitch:delegate --auto <task>`

This command should be treated as the primary execution entrypoint.

Related command:

- `/task-route`: stores task-type routing preferences

## Supported forms

### Default implementation route

```text
/delegate <task>
```

Resolved target:

```text
Sonnet
```

This mode follows the current active member of the `Sonnet` group.

### Explicit Sonnet member

```text
/delegate --model <member> <task>
```

Examples:

```text
/delegate --model gpt-5.4 修复当前任务
/delegate --model qwen3.6-plus 调查当前实现差异
```

Resolved target:

```text
Sonnet/<member>
```

This mode should not require changing the current active member first.

### Explicit route target

```text
/delegate --to <group>|<group/member> <task>
```

Examples:

```text
/delegate --to Sonnet 修复当前问题
/delegate --to Sonnet/gpt-5.4 审查当前改动风险
```

Resolved target:

```text
<group>
```

or:

```text
<group>/<member>
```

This is the preferred explicit-routing form.

### Automatic routing mode

```text
/delegate --auto <task>
```

Examples:

```text
/delegate --auto 分析当前路由问题，完成修复并检查风险
/delegate --auto 搜索支付相关入口并汇总影响范围
```

This mode should:

- classify the task
- decide whether single-agent or multi-agent execution is needed
- consult `/task-route` preferences if available
- produce a route-aware execution plan
- then execute or propose execution using the selected routes

## Direct execution behavior

When invoked as a project-local command, execute immediately using the current thread context.

### Parse rules

#### Form A

```text
/delegate <task>
```

Resolve target as `Sonnet`.

#### Form B

```text
/delegate --model <member> <task>
```

Resolve target as `Sonnet/<member>`.

#### Form C

```text
/delegate --to <group>|<group/member> <task>
```

Resolve target exactly as provided.

#### Form D

```text
/delegate --auto <task>
```

Resolve via task classification plus route preferences.

### Runtime behavior

For direct routing forms:

1. Parse arguments and determine the resolved target route.
2. Use the current conversation context and approved plan as execution input.
3. Delegate the task to a subagent using the resolved target route as the model path.
4. Instruct the subagent:
   - execute within scope
   - do not re-plan unless blocked
   - return only the structured execution summary

For `--auto`:

1. Classify the task.
2. Check `/task-route` preferences if they exist.
3. Choose single-agent or multi-agent execution.
4. Assign one or more route targets.
5. Execute through the chosen routed worker sequence.

## Required result format

The delegated subagent should return only:

- summary
- files changed
- commands run
- test results
- unresolved risks

## Recommended subagent prompt template

Use this worker prompt body when `/delegate` resolves a route:

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

## Recommended route-aware wrapper

Prepend a short wrapper when implementing the command runner:

```text
Execute this task using route: <resolved-target>.
Treat the route as fixed for this task.
```

Examples:

```text
Execute this task using route: Sonnet.
Treat the route as fixed for this task.
```

```text
Execute this task using route: Sonnet/gpt-5.4.
Treat the route as fixed for this task.
```

## Failure handling

If the resolved target is `Sonnet` but the project does not define a `Sonnet` group in OctoSwitch:

- stop
- explain that `Sonnet` is missing
- suggest either creating a `Sonnet` group or using `/delegate --to <existing-group>/<member> ...`

If the user supplies `--model <member>` but that member does not exist under `Sonnet`, report the routing error directly.

If the user supplies `--to`, do not reinterpret or rewrite that explicit route target.

If the user supplies `--auto`, prefer task-route preferences over hardcoded defaults when those preferences exist.

## Practical examples

```text
/delegate 按当前确认方案完成实现并测试
/delegate --model gpt-5.4 修复当前 bug 并汇报测试结果
/delegate --to Sonnet/gpt-5.4 审查当前改动风险
/delegate --auto 分析当前 bug，完成修复并做回归检查
```
