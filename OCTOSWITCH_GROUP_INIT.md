# OctoSwitch 分组初始化方案

这份方案用于把当前 Claude Code routing skills 和 OctoSwitch 现有分组体系对齐。

目标是先用你已经在服务里存在的分组别名：

- `Sonnet`
- `Opus`
- `Haiku`

这样可以立即支持：

- `/show-routing`
- `/route-activate <group> <member>`
- `/subagent-model <member>`
- `/delegate <task>`
- `/delegate --model <member> <task>`
- `/delegate --to <group>|<group/member> <task>`

## 推荐语义

建议先把分组语义固定下来：

- `Sonnet`
  - 默认实现组
  - 用于 coding、bugfix、refactor、直接执行任务
- `Opus`
  - 默认审查组
  - 用于 review、risk check、方案把关、测试缺口检查
- `Haiku`
  - 默认检索组
  - 用于 search、快速定位、低成本信息收集

这是一套“按任务角色映射到现有别名”的方案，不要求你额外创建 `executor/reviewer/searcher`。

## 推荐初始成员

按你当前服务返回的数据，建议先这样用：

- `Sonnet` -> `gpt-5.4`
- `Opus` -> `gpt-5.4`
- `Haiku` -> `MiniMax-M2.7`

如果后续你往这些组里再加更多成员，skills 仍然可以继续工作。

## 推荐 task-route 初始配置

如果后续实现 task preference 存储，推荐先使用这组三条：

```text
/task-route implementation --target Sonnet/gpt-5.4
/task-route review --target Opus/gpt-5.4
/task-route search --target Haiku/MiniMax-M2.7
```

对应语义：

- implementation -> `Sonnet/gpt-5.4`
- review -> `Opus/gpt-5.4`
- search -> `Haiku/MiniMax-M2.7`

## 推荐 delegate-auto 决策基线

自动委派建议先按下面的基线：

- 小型实现任务
  - 直接走 `Sonnet/gpt-5.4`
- 搜索型任务
  - 走 `Haiku/MiniMax-M2.7`
- 审查型任务
  - 走 `Opus/gpt-5.4`
- 混合任务
  - 先 `Haiku`
  - 再 `Sonnet`
  - 最后 `Opus`

## 为什么先用这套方案

优点：

- 不需要重建你当前已有分组
- 和现有技能命令更容易对齐
- 显式 `group/member` 路由可以立刻工作
- 后续如果要升级成角色组，也可以平滑迁移

## 后续可选升级

如果后面你想把语义再做得更“角色化”，可以再新增：

- `executor`
- `reviewer`
- `searcher`

然后把它们分别映射到不同上游模型。

但在当前阶段，没有必要为了命名统一而重做分组。先让 `Sonnet / Opus / Haiku` 成为稳定任务入口，实际收益更大。
