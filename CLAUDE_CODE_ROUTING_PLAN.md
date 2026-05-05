# Claude Code × OctoSwitch 路由插件方案

## 1. 目标

本方案希望实现一套稳定的双层工作流：

- Claude Code 主线程使用模型 A，负责规划、决策、汇总
- Claude Code subagent 使用模型 B，负责执行任务
- 执行模型可按默认规则切换
- 单个任务可临时指定模型
- 路由状态尽量由 OctoSwitch 统一管理
- 后续可从“模型切换器”演进到“任务编排器”

本方案以 **方案 B：Claude Code 路由插件 + OctoSwitch 控制平面** 为基础。

---

## 2. 当前结论

### 2.1 已确认结论

1. Claude Code 的 subagent 已成功使用自定义模型名：

```text
model: executor/sonnet
```

2. 这说明在当前接入方式下，Claude Code subagent 可以消费 OctoSwitch 暴露的 `group/member` 模型路径。

3. 因此以下两种模式都具备可行性：

- 默认模式：subagent 使用 `executor`
- 覆盖模式：subagent 使用 `executor/sonnet`、`executor/opus`、`executor/haiku`

4. OctoSwitch 现有能力与该方案天然契合，因为它已经支持：

- 模型分组 `group alias`
- 活动成员 `active binding`
- `group/member` 路径式模型
- 本地网关统一入口

---

### 2.2 关键判断

本方案最合理的落点不是“只做一个 Claude Code 小插件”，而是：

**让 Claude Code 插件作为交互入口，让 OctoSwitch 成为模型路由控制平面。**

也就是：

- Claude Code 插件负责命令入口和 subagent 调用
- OctoSwitch 负责默认路由状态、组成员切换、任务推荐和后续可观测性

---

## 3. 核心设计原则

### 3.1 双模式并存

系统必须同时支持：

- 默认路由模式
- 单任务显式覆盖模式

原因：

- 默认模式适合高频日常使用
- 覆盖模式适合一个会话中不同任务使用不同模型

---

### 3.2 OctoSwitch 统一保存默认状态

默认执行模型不应只保存在 Claude Code 插件内部状态文件中，而应尽量由 OctoSwitch 保存和展示。

这样可以获得：

- 路由状态统一
- UI 可视化
- 多客户端共享
- 后续 metrics / fallback 可扩展

---

### 3.3 执行类任务走显式 model 路由

当任务需要精确指定执行模型时，应优先使用：

- `executor/sonnet`
- `executor/opus`
- `executor/haiku`

而不是先切全局 active binding 再执行。

这样可以避免：

- 受默认 active binding 干扰
- 影响其他客户端或其他会话

---

### 3.4 通信优先采用本地 HTTP API

Claude Code 插件与 OctoSwitch 之间，优先使用本地 HTTP API 通信。

不推荐第一版采用：

- 直接改本地数据库
- 直接改配置文件
- 依赖桌面端专有 IPC

---

## 4. 模型路由模型

### 4.1 建议的角色分组

建议在 OctoSwitch 中至少建立两个 group：

- `planner`
- `executor`

后续可扩展：

- `reviewer`
- `searcher`

---

### 4.2 路由语义

#### 裸 group

例如：

```text
executor
```

语义：

- 使用该 group 当前 active binding

特点：

- 会受 active binding 影响

#### 显式 group/member

例如：

```text
executor/sonnet
executor/opus
```

语义：

- 直接路由到指定成员

特点：

- 不受 active binding 影响

---

### 4.3 路由优先级

推荐优先级：

1. 显式任务目标
   - `/delegate-to executor/sonnet ...`
   - `/delegate --model sonnet ...`
2. 任务类型推荐规则
   - `implementation -> executor/sonnet`
3. group 默认 active binding
   - `executor`

---

## 5. 命令体系

本方案建议把命令分为四类：

- 状态查询
- 激活切换
- 任务委派
- 任务推荐 / 自动编排

---

### 5.1 状态查询命令

#### `/show-routing`

用途：

- 查看当前路由状态

建议输出：

- planner 当前 active member
- executor 当前 active member
- executor 可用成员
- 是否启用 `group/member` 路径模型
- OctoSwitch 是否在线

#### `/subagent-model`

当不带参数时，可退化为查询当前默认执行模型。

---

### 5.2 激活切换命令

#### `/subagent-model <member>`

用途：

- 设置默认执行模型

语义：

- 将 `executor` group 的 active binding 切换到指定 member

示例：

```text
/subagent-model sonnet
```

#### `/route-activate <group> <member>`

用途：

- 通用切换命令

语义：

- 切换任意 group 的 active binding

示例：

```text
/route-activate executor sonnet
/route-activate planner opus
```

#### `/planner-model <member>`（可选）

用途：

- 切换 `planner` 默认成员

MVP 阶段可不做。

---

### 5.3 任务委派命令

#### `/delegate <task>`

用途：

- 使用默认执行路由完成任务

实际目标：

```text
executor
```

特点：

- 会受 `executor.active_binding` 影响

#### `/delegate --model <member> <task>`

用途：

- 为单个任务临时指定执行模型

示例：

```text
/delegate --model sonnet 修复任务A
/delegate --model opus 重构任务B
```

实际目标：

- `executor/sonnet`
- `executor/opus`

特点：

- 不修改默认 active binding
- 不受 active binding 影响

#### `/delegate-to <group> <task>`

用途：

- 将任务显式交给某个 group

示例：

```text
/delegate-to executor 按当前方案完成实现并测试
/delegate-to reviewer 审查当前改动风险
```

#### `/delegate-to <group>/<member> <task>`

用途：

- 将任务显式交给某个 group/member

示例：

```text
/delegate-to executor/sonnet 修复任务A
/delegate-to searcher/haiku 汇总任务C
```

---

### 5.4 任务推荐与自动编排命令

#### `/task-route <task-kind> --target <group>/<member>`

用途：

- 为某类任务设置推荐目标

示例：

```text
/task-route implementation --target executor/sonnet
/task-route review --target reviewer/opus
/task-route search --target searcher/haiku
```

#### `/task-route <task-kind> --model <member>`

用途：

- 为某类任务设置推荐执行模型

示例：

```text
/task-route implementation --model sonnet
```

等价于：

```text
implementation -> executor/sonnet
```

#### `/delegate-auto <task>`

用途：

- 由路由层根据任务内容自动判断：
  - 任务类型
  - subagent 数量
  - subagent 角色
  - subagent 模型

示例：

```text
/delegate-auto 分析网关路由问题并给出修复方案，然后完成实现和测试
```

MVP 阶段建议先不实现完整自动编排，但应预留设计。

---

## 6. OctoSwitch 通信设计

### 6.1 通信方式

推荐使用：

- 本地 HTTP API

推荐的插件最小配置：

```json
{
  "octoswitch_base_url": "http://127.0.0.1:8787",
  "default_executor_group": "executor",
  "default_planner_group": "planner"
}
```

---

### 6.2 通信职责划分

#### 配置类动作通过 HTTP API

例如：

- 查询当前路由状态
- 切换某个 group 的 active binding
- 查询成员列表
- 设置任务推荐规则

#### 执行类动作通过 Claude Code subagent 完成

例如：

- 把任务交给 `executor`
- 把任务交给 `executor/sonnet`
- 把任务交给 `reviewer/opus`

也就是说：

- OctoSwitch 负责“控制平面”
- Claude Code subagent 负责“执行平面”

---

### 6.3 典型通信流程

#### 查询状态

- 命令：
  - `/show-routing`
  - `/subagent-model`
- 插件调用：
  - `GET /v1/routing/status`

#### 切换默认执行模型

- 命令：
  - `/subagent-model sonnet`
  - `/route-activate executor sonnet`
- 插件调用：
  - `POST /v1/routing/groups/executor/active-member`

#### 设置任务推荐规则

- 命令：
  - `/task-route implementation --target executor/sonnet`
- 插件调用：
  - `POST /v1/routing/task-preferences`

#### 自动编排建议

- 命令：
  - `/delegate-auto <task>`
- 插件调用：
  - `POST /v1/routing/plan`

---

### 6.4 错误处理

插件应统一处理以下错误：

- OctoSwitch 未启动
- HTTP 超时
- group 不存在
- member 不存在
- `group/member` 路径模型未开启
- 返回结构不符合预期

建议错误输出格式：

```text
Route switch failed: executor member 'sonnet' was not found.
Check whether the executor group exists and contains that member in OctoSwitch.
```

---

## 7. OctoSwitch 侧建议新增 API

### 7.1 路由状态接口

```text
GET /v1/routing/status
```

用途：

- 返回 planner / executor 当前 active member
- 返回成员列表
- 返回 `allow_group_member_model_path`

---

### 7.2 切换 active member 接口

```text
POST /v1/routing/groups/:alias/active-member
```

请求体示例：

```json
{
  "member": "sonnet"
}
```

用途：

- 支撑 `/subagent-model`
- 支撑 `/route-activate`

---

### 7.3 group 成员列表接口

```text
GET /v1/routing/groups/:alias/members
```

用途：

- 让插件查询 executor / planner / reviewer 等 group 下可用成员

---

### 7.4 健康检查接口

```text
GET /healthz
```

或：

```text
GET /v1/ping
```

用途：

- 快速确认 OctoSwitch 是否在线

---

### 7.5 任务推荐规则接口

```text
GET /v1/routing/task-preferences
POST /v1/routing/task-preferences
```

请求体示例：

```json
{
  "task_kind": "implementation",
  "target": "executor/sonnet"
}
```

用途：

- 保存任务类型到目标模型的推荐映射

---

### 7.6 自动编排建议接口

```text
POST /v1/routing/plan
```

请求体示例：

```json
{
  "task": "分析网关路由问题并给出修复方案，然后完成实现和测试",
  "source": "claude-code-plugin"
}
```

用途：

- 返回建议的 subagent 数量、角色和目标模型

MVP 阶段可以先不实现。

---

### 7.7 `subagent` 路由增强

已有接口：

```text
/v1/subagent/run
```

建议后续支持附加字段：

```json
{
  "model": "executor/sonnet",
  "role": "executor",
  "source": "claude-code-plugin",
  "task_type": "implementation"
}
```

用途：

- 便于 metrics 分类
- 便于未来 fallback / 策略路由

---

## 8. Claude Code 侧 Skill 设计

建议把能力拆成以下 skill。

---

### 8.1 `routing-status`

职责：

- 查询并展示当前路由状态

服务命令：

- `/show-routing`
- `/subagent-model` 无参数

---

### 8.2 `route-activation`

职责：

- 切换某个 group 的 active binding

服务命令：

- `/subagent-model <member>`
- `/route-activate <group> <member>`
- `/planner-model <member>`

---

### 8.3 `delegate-routing`

职责：

- 把用户任务解析为具体目标模型
- 调度 Claude Code subagent 执行

服务命令：

- `/delegate`
- `/delegate --model`
- `/delegate-to`

---

### 8.4 `task-routing-preferences`

职责：

- 管理任务类型到推荐目标的映射

服务命令：

- `/task-route`

---

### 8.5 `auto-routing-orchestrator`

职责：

- 自动决定 subagent 数量、类型和目标模型

服务命令：

- `/delegate-auto`

MVP 阶段先保留设计，不必完整实现。

---

### 8.6 `subagent-result-normalizer`

职责：

- 统一 subagent 输出格式

标准输出建议：

- summary
- files changed
- commands run
- test results
- unresolved risks

这样主线程更容易消费不同模型和不同角色的结果。

---

## 9. 插件内部模块划分

如果最终以插件形态实现，建议拆分为：

- `config`
  - 读取 OctoSwitch 地址和默认 group
- `octoswitch-client`
  - 封装 HTTP API
- `route-resolver`
  - 解析显式目标、任务推荐和默认 active binding
- `delegate-runner`
  - 实际触发 Claude Code subagent
- `result-normalizer`
  - 统一输出
- `commands`
  - 对外暴露命令入口

---

## 10. MVP 范围

MVP 只需证明三件事：

1. 主线程 A、执行线程 B 工作流可跑通
2. 默认执行模型可切换
3. 单任务执行模型可覆盖

因此 MVP 建议只做：

- `planner` / `executor` 两个基础 group
- `executor` 的多个成员
- `group/member` 路径模型
- `/subagent-model`
- `/delegate`
- `/delegate --model`
- `/show-routing`
- `GET /v1/routing/status`
- `POST /v1/routing/groups/:alias/active-member`

MVP 不建议一开始就做：

- 自动接管所有系统 subagent
- 复杂多角色策略
- 自动 fallback
- 大规模 UI 改造
- 完整自动编排

---

## 11. MVP 实施步骤

### 阶段 0：基础准备

1. 确认 OctoSwitch 本地网关稳定运行
2. 确认 `allow_group_member_model_path = true`
3. 确认 Claude Code 请求已通过 OctoSwitch

---

### 阶段 1：建立基础路由

1. 创建 `planner` group
2. 创建 `executor` group
3. 为 `executor` 添加：
   - `sonnet`
   - `opus`
   - `haiku`
4. 配置 `planner` 的主线程模型
5. 设置 `executor` 默认 active binding

验收：

- `GET /v1/models` 能看到：
  - `planner`
  - `executor`
  - `executor/sonnet`
  - `executor/opus`
  - `executor/haiku`

---

### 阶段 2：人工验证

验证项：

1. Claude Code subagent 能运行 `model: executor/sonnet`
2. 默认 `executor` 是否跟随 active binding
3. `executor/opus` / `executor/haiku` 是否可用
4. 单任务显式覆盖是否不受 active binding 影响

当前已确认：

- `model: executor/sonnet` 已成功跑通

---

### 阶段 3：补最小控制接口

优先新增：

1. `GET /v1/routing/status`
2. `POST /v1/routing/groups/executor/active-member`

可选新增：

3. `GET /v1/routing/groups/executor/members`
4. `GET /healthz`

---

### 阶段 4：实现插件 MVP

先实现：

1. `/subagent-model`
2. `/delegate`
3. `/show-routing`

如资源允许，可同阶段补：

4. `/route-activate`
5. `/delegate-to`

---

### 阶段 5：补统一结果格式

在执行 subagent 提示词中统一要求返回：

- summary
- files changed
- commands run
- test results
- unresolved risks

---

### 阶段 6：进入第二阶段增强

在 MVP 成立后，再考虑：

1. `/task-route`
2. `task-preferences` 接口
3. reviewer / searcher 角色
4. `/delegate-auto`
5. `/v1/routing/plan`
6. `/v1/subagent/run` 的角色标签和 metrics

---

## 12. 推荐优先级

### P0

- 建立 `planner` / `executor`
- 打通 `executor/member` 路由
- 验证 subagent 使用自定义模型名

### P1

- `routing/status`
- `set active member`
- `/subagent-model`
- `/delegate`
- `/show-routing`

### P2

- `/route-activate`
- `/delegate-to`
- group 成员列表接口
- 更完整错误处理

### P3

- `/task-route`
- 任务推荐规则接口
- reviewer / searcher 角色

### P4

- `/delegate-auto`
- 自动编排建议接口
- `subagent` 路由增强和 metrics 标签

---

## 13. 最终建议

最值得先做的不是复杂自动编排，而是把下面这条链路先打通：

1. OctoSwitch 中建立 `planner` / `executor`
2. Claude Code subagent 使用 `executor/member`
3. 默认执行模型由 OctoSwitch active binding 管理
4. 单任务执行模型由显式 `executor/member` 覆盖
5. Claude Code 插件只负责：
   - 查状态
   - 切默认
   - 发委派

这条链路一旦稳定，后续再往任务推荐、自动编排、角色化路由扩展，就会顺很多。
