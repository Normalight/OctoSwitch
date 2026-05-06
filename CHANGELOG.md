# Changelog

## [v0.5.2] — 2026-05-05

### 用量统计

- 趋势图零值补齐，空时间段不再缺失数据点
- 请求日志 Input / Cache 按钮显示，tooltip 分离
- 表格表头背景、滚动条主题、列宽优化

---

## [v0.5.1] — 2026-05-05

### 更新检查去 API 化

- **无需 GitHub 登录**：用 `releases/latest` 重定向获取最新 tag，替换 `api.github.com`
- **安装包定位**：HEAD 请求已知文件名模式（`.dmg`/`.AppImage`/`.exe` 等）
- **更新日志**：直接从 raw `CHANGELOG.md` 提取，不依赖 API
- 任何人不再受 403/rate-limit 限制

### 安装脚本 & UI 修复

- macOS 安装前先 kill 旧实例，避免副本冲突
- SaveIndicator 按钮布局修复

---

## [v0.5.0] — 2026-05-05

### 网关 & 路由

- **智能模型路由**：新增 `/reload` 技能支持运行时热重载路由配置
- **路由器增强**：扩展路由端点，优化请求分发逻辑
- **Copilot 转发**：重构 Copilot 请求处理，提升兼容性和稳定性
- **转发层精简**：移除冗余代码路径，统一 streaming / non-streaming 处理

### 数据库 & 迁移

- **性能优化**：重构 model binding DAO（382 行），大幅提升查询效率
- **迁移 008**：清理废弃字段（`drop_total_cost`），精简表结构
- **迁移 009/010**：新增 `metrics_snapshots` 表 + 索引，支撑用量统计持久化
- **清理**：移除 model group / member DAO 中的冗余逻辑

### 用量统计

- **快照持久化**：新增 metrics_snapshots 表，用量数据跨重启保留
- **聚合优化**：重构 metrics_aggregator（117 行），支持更长窗口的统计
- **趋势图简化**：只保留总 Token 消耗曲线，移除分项 Input/Output 开关
- **请求日志虚拟滚动**：引入 `@tanstack/react-virtual`，500 条日志流畅滚动

### 命令层

- **统一错误处理**：`AppError` 扩展 40 行，覆盖更多错误场景
- **路由调试面板**：增强 routing debug 功能
- **命令精简**：移除 10+ 处不必要的中间变量和冗余日志
- **测试工具**：新增 `test_utils` 模块（86 行），复用测试基础设施

### UI & 组件

- **ModelStack 虚拟列表**：Provider 列表改用虚拟滚动，大幅提升大列表性能
- **保存指示器**：Provider 编辑等操作增加即时保存状态反馈
- **样式一致性**：统一多处 CSS token 引用（`--radius-sm`、`--radius-pill`）
- **清理**：移除 en/zh-CN 中的 10 个未使用 i18n key

### 平台 & 构建

- **Python 脚本**：新增 `octoswitch_routing.py` 路由控制脚本
- **依赖更新**：`@tanstack/react-virtual` 3.x，reqwest 等

---

## [v0.4.13] — 2026-05-05

### Modal 层级修复

- Modal z-index 自动递增，确保多层弹窗正确堆叠

### 趋势图简化

- 趋势图中只显示消耗总 Token，移除分项 Input/Output 开关曲线
- SaveIndicator onDone 回调更稳定

### 代码清理

- 移除重复 import
- 清理无引用 CSS

---

## [v0.4.12] — 2026-05-05

### CI / 工作流

- 更新 CI 和 Release 工作流配置

---

## [v0.4.11] — 2026-05-05

### 系统代理自动检测

- reqwest 启用 `system-proxy` feature，自动检测 macOS 系统代理（Clash 等）
- 代理优先级：应用配置 > HTTPS_PROXY > HTTP_PROXY > 系统代理

### 更新下载超时保护

- preparing 状态增加 5 秒超时，超时后回到 checked 状态
- 修复从非 checked 状态启动下载时状态丢失问题

---

## [v0.4.10] — 2026-05-05

### 用量统计优化

- **图表逐模型细分**：tooltip 悬停显示每个分组的 Input (xx% 缓存) / Output 明细
- **移除错误率**：KPI 卡片去���错误率，只保留消耗 Token / Input / Output / 缓存
- **列名优化**：请求表格 Input 列显示 input + cache 合计值，列名 "Input + Cache read"
- **高度统一**：用量页时间选择框和刷新按钮统一 40px 高度

### Select 下拉框修复

- chevron 箭头修复：`.settings-lang-select` background shorthand → background-color
- 全局 `background-size: 0.75rem` 确保 SVG 箭头不拉伸

---

## [v0.4.9] — 2026-05-05

### 网关管理优化

- **状态合并**：网关状态（绿/红圆点）并入网关配置卡片头部，不再单独占一张卡片
- **重启按钮**：网关未运行时显示「重启网关」按钮，一键通过 restart channel 重启
- **新增 `restart_gateway` 命令**：Rust 后端通过已有 restart_tx channel 发送当前配置重启
- **精简说明**：移除冗长的 5 步排查折叠块，保留错误信息 + 重启入口

---

## [v0.4.8] — 2026-05-05

### 更新体验优化

- **即时反馈**：点击「下载并安装」后按钮立刻切换为「正在准备下载...」spinner，不等待后端响应
- **进度条丝滑**：用 `tokio::select!` 120ms 间隔 tick 替代百分比阈值，下载条持续更新不再卡顿
- **降级弹窗**：下载失败（无安装包、网络错误等）弹出确认框，可选择「在浏览器中打开」跳转 GitHub Release 页
- **刷新按钮**：版本信息行右侧新增矢量旋转图标，点击强制重新检查（绕过 60 秒缓存）
- **状态缓存**：切换标签页后再回到关于，不重复请求 GitHub API；下载中途切出不中断状态

### 代理支持

- 更新检查和下载统一使用共享 `state.http_client`（已配 proxy），之前检查更新用裸 client 不走代理
- 安装脚本支持 `HTTP_PROXY` / `http_proxy` 环境变量

### macOS 托盘行为

- 左键点击托盘图标显示右键菜单（macOS 原生惯例），不再直接打开主窗口
- Windows/Linux 保持原有左键打开窗口行为

### 安装脚本

- DMG 卷路径解析修复（`hdiutil attach` stderr + tab 切分）
- aarch64 asset 匹配（Tauri 构建用 `aarch64` 而非 `arm64`）

### UI

- Select 下拉框全局 `appearance: none` + 自定义 SVG chevron，统一 `min-height: 44px`

---

## [v0.4.7] — 2026-05-05

### Select 下拉框高度修复

- **`appearance: none` + 自定义箭头**：全局 `<select>` 添加 `appearance: none`，使用内联 SVG 箭头替代原生渲染，彻底解决 macOS WebKit 不遵循 `min-height`/`padding` 的问题
- **统一 `min-height: 44px`**：设置语言/日志级别/分组管理添加成员/路由调试/用量页等所有下拉框
- **箭头留白**：右侧 `padding-right: 32px` 为自定义 chevron 留出空间
- **light mode**：浅色主题使用更浅的箭头颜色

### 安装脚本修复

- **DMG 卷路径解析**：`hdiutil attach` 输出到 stderr，改用 `2>&1 | rev | cut -f1 | rev` 按 tab 切分取最后字段
- **Proxy 支持**：自动读取 `HTTP_PROXY`/`http_proxy` 环境变量
- **aarch64 asset**：新增 `_aarch64.dmg`、`_aarch64.app.tar.gz` 模式（Tauri 构建用 `aarch64` 而非 `arm64`）
- **API 限流友好**：不再对空响应抛 JSON 解析错，静默回退到直接 URL 试探

---

## [v0.4.6] — 2026-05-05

### 下载安装进度反馈增强

- **阶段指示器**：下载完成显示绿色对勾 → 正在安装/启动安装程序时显示旋转图标，视觉上清晰区分各阶段
- **进度条优化**：百分比左对齐、文件大小右对齐，徽章添加呼吸脉冲动画
- **新增安装启动事件**：监听后端 `update-installer-launching` 事件，显示「正在启动安装程序...」阶段
- **重启提示**：安装和启动阶段底部统一显示「安装完成后应用将自动重启」

### README 文档更新

- **安装指南重构**：新增快速脚本安装、手动下载表格（macOS/Linux/Windows 包类型）、应用内更新三个小节
- **macOS Gatekeeper**：注明 install.sh 已自动处理，手动命令作为备选
- **依赖说明**：区分最终用户（无前置依赖）和开发者
- **分组页**：提及 CC Switch 内联按钮
- **未来特性**：新增分级请求超时

### 其他

- `.opencode/` 加入 `.gitignore`
- UI 控件 `min-height` 统一调整为 40px

---

## [v0.4.5] — 2026-05-05

### 跨平台安装与自动更新

- **跨平台安装脚本**：`scripts/install.sh` 自动检测 OS 并下载对应安装包。macOS 自动挂载 DMG → 复制 .app → 移除 quarantine → ad-hoc 签名绕过 Gatekeeper。API 限流时自动回退到直接 URL 模式。
- **更新系统重构**：平台感知的 `pick_installer_asset` 检测 `.dmg`/`.app.tar.gz`（macOS）、`.AppImage`/`.deb`（Linux）、`.exe`/`.msi`（Windows）。macOS 更新安装完成后自动调用 `open -n -a` 启动新版本并 `std::process::exit(0)` 退出旧进程。
- **Linux 编译修复**：`path.to_string_lossy()` 返回 `Cow<str>`，数组字面量中 `&Cow<str>` 与 `&str` 类型不匹配，使用 `.as_ref()` 统一类型。

### UI 优化

- **统一控件高度**：input、select、button 统一 `min-height: 36px`
- **Skills 插件模态简化**：移除文件同步逻辑，改为纯仓库链接引导模式
- **CC Switch 按钮内联**：分组标签页新增「注册到 CC Switch」按钮，与「添加分组」并列

---

## [v0.4.4] — 2026-05-05

### DeepSeek V4 reasoning_content 兼容

- **`reasoning_content` 往返保留**：Anthropic↔OpenAI 格式转换时，对 DeepSeek/Moonshot/Kimi 等需要 `reasoning_content` 的 provider，自动将 thinking blocks 转换为 `reasoning_content` 字段（非标准 `reasoning_text`），满足 tool-call 回传要求。
- **自动检测**：`is_reasoning_content_provider()` 同时检查 provider 的 base_url 和模型名，包含 `deepseek`/`moonshot`/`kimi` 即启用。覆盖 OpenCodeGo + deepseek-v4-pro 场景（base_url 不含 deepseek 但模型名含）。
- **流式同步适配**：SSE 流式转发中同时检测 delta 的 `reasoning_content` 和 `reasoning_text`，映射为 Anthropic thinking block。

### URL 路径去重

- **`deduplicate_url_path()`**：修复 provider base_url 含 `/v1` 且转换时又追加 `/v1/chat/completions` 导致的双重路径问题（`/v1/v1/chat/completions` → `/v1/chat/completions`）。

### 禁用分组网关隔离

- `GET /v1/routing/status` — 过滤 `is_enabled=false` 的分组
- `GET /v1/routing/groups/:alias/members` — 禁用分组返回 403
- `PUT /v1/routing/groups/:alias/active-member` — 禁用分组返回 403

### CC Switch 深度链接注册

- 分组标签页新增「注册到 CC Switch」按钮，自动检测 cc-switch 中是否已有 OctoSwitch provider
- 生成完整 `ccswitch://` URL，按 Sonnet/Haiku/Opus 分组别名自动映射模型参数
- endpoint 使用裸地址（不含 `/v1`），由 cc-switch 负责路径拼接

### Provider 层重构

- `ProviderSummary` 轻量类型，list/create/update 返回摘要信息
- 新增 `get_provider` 命令按需获取完整 Provider（含 api_key_ref）

### 错误处理增强

- `RoutingError` 结构化错误类型
- `AppError::ModelGroupDisabled` 适配网关 403 响应
- DAO 层统一返回 `AppError` 而非 String

---


### CC Switch deep link integration

- **`ccswitch://` provider registration**: Skills page now generates a `ccswitch://` deep link that registers OctoSwitch gateway as a provider in CC Switch (`ccswitch://v1/import?resource=provider&...`). Clicking the button opens CC Switch with a confirmation dialog, enabling one-click provider setup.
- **`ccswitch://` skill repo registration**: A second deep link registers the `Normalight/OctoSwitch` repo as a skill source in CC Switch, so users can discover and install OctoSwitch's built-in skills from the GitHub repo.
- **`open_cc_switch_deeplink` command**: Validates that only `ccswitch://` URLs can be opened, preventing arbitrary URL injection.
- **Unit tested**: Deep link generation logic covered by 4 Rust unit tests verifying URL formatting, percent-encoding, and validation.

### Provider layer refactoring

- **`ProviderSummary` type**: List/create/update commands now return a lightweight summary type (name + api_key_ref + endpoint) instead of the full Provider, improving frontend performance and reducing serialization overhead.
- **New `get_provider` command**: Fetch full Provider details (including api_key_ref) by ID, used when opening the edit modal. Previously the edit modal relied on list data which lacked the full api_key_ref.
- **Frontend hooks updated**: `useProviders` hook now returns `ProviderSummary[]` and fetches full `Provider` on demand via the new `getProvider` API.

### Error handling improvements

- **New `model_slug` domain**: Input validation for model names and group aliases — rejects slashes and empty values with structured error messages. Applied across model binding, model group, and routing service.
- **Routing service errors**: Replaced generic string errors with typed `RoutingError` (not_found, model_not_bound, disabled, invalid_spec) carrying structured context and model name display.
- **Gateway `ForwardRequestError`**: New `From<AppError>` implementation preserving structured error details through the axum HTTP layer, giving clients consistent JSON error responses.
- **DAO error mapping**: Task route preference and model fetch DAOs now return `AppError` instead of raw `String`, with proper error conversion via `thiserror`.

### Gateway improvements

- **Router enhancements**: Added `/v1/routing/status` endpoint (lists all groups with members and active bindings), `GET /v1/routing/groups/:alias/members`, `PUT /v1/routing/groups/:alias/active-member`, and `/v1/plugin/config` with runtime plugin configuration.
- **Copilot streaming optimization**: Simplified request translation and stream processing in the Copilot forwarder, reducing redundant method chains.
- **Copilot account DAO**: Added vendor caching (`copilot_vendor_cache`) and lifecycle management in `copilot_account_dao` — accounts now have `updated_at`, `token_expires_at`, and are filtered by provider association.

### Database layer refactoring

- **Provider DAO**: Insert with explicit ID, consistent error handling via `AppError`, cascading deletes on provider removal.
- **Model binding DAO**: Sorted by provider + model_name, full text search across model_name and upstream_model_name, pagination support, cache token tracking.
- **Model group DAO**: Case-insensitive alias lookup, membership validation, auto-clear active binding when member is removed.
- **Model group member DAO**: Batch operations for group-bindings, catalog query building, membership count tracking.

### Other improvements

- **Circuit breaker**: Added `mark_success` to reset failure count on healthy responses, with cooldown-based auto-recovery and per-provider isolation.
- **Config import**: Improved import deduplication and merge logic across providers, bindings, groups, and task-route preferences.
- **Migration system**: Added migration 006 (`delegate_agent_kind`) and 007 (`delegate_model`) for extended task preference metadata.

---


### Delegate composite skill system

- **Plan-first execution enhanced**: `/delegate` now registers tasks in TodoWrite for central progress tracking, validates plans before dispatch, and uses wave-based scheduling with explicit dependency graphs.
- **Two-stage review gates**: Serial tasks pass through spec-compliance and code-quality checks before dependent tasks launch. If criteria are not met, the task is retried with specific feedback (max 2 retries).
- **Structured status protocol**: Workers report DONE / DONE_WITH_CONCERNS / BLOCKED / NEEDS_CONTEXT, enabling consistent handling by the controller.
- **Verification-before-completion**: Controller verifies `doneWhen` criteria against actual file changes, not just worker claims. No "all tasks complete" until every criterion is verified.
- **Stop-on-blocker discipline**: When a task is BLOCKED, dependent work halts immediately — no silent fallback or skipping.
- **Composite skill architecture**: Delegate now orchestrates sub-skills (`verify` for verification gates, `worker` for structured response protocol) for disciplined execution, inspired by superpowers patterns.

### Skill docs improvements

- `delegate/verify/SKILL.md` — new verification skill with per-criterion checking, evidence tracking, and PROCEED/RETRY/ESCALATE recommendations.
- `delegate/worker/SKILL.md` — new worker protocol skill defining required response sections (route, status, summary, files, commands, tests, risks).
- `delegate/SKILL.md` — restructured with controller-subagent separation, fresh context per task, model selection by complexity, and TodoWrite integration.

---

## [v0.3.3] — 2026-04-14

### Bug fixes

- **Skills marketplace path**: Fixed "Failed to read marketplace manifest" error on release builds. `CARGO_MANIFEST_DIR` resolves to the CI build path at compile time, which doesn't exist at runtime. Now gracefully handles missing manifest and returns installed plugin info.
- **External URL opening**: GitHub release page now opens via Tauri opener API instead of blocked `window.open()` in webview.

### Build

- Fixed `tauri.conf.json` version mismatch — this file is used by Tauri build for output filenames and must match other version sources.

---

## [v0.3.2] — 2026-04-13

### In-app update

Clicking **Update** in Settings now downloads the installer silently with progress bar, runs the NSIS installer in silent mode, and restarts the app automatically. No more manual browser download.

### Delegate routing improvements

- Task analysis phase: the main model evaluates the request and chooses among serial, parallel, or single-agent strategies
- Parallel dispatch: independent subtasks spawn separate agents simultaneously, each reporting results as they complete
- Progressive reporting: real-time per-agent output followed by a unified summary
- Simplified command surface: only `/delegate <task>` and `/delegate --to <group> <task>`

### Skills page & preferences

- Redesigned Skills tab with improved layout for managing task-route preferences
- Delete confirmation modal replaces browser-native confirm dialog
- Auto-sync: preference changes automatically regenerate agents and sync to both cc-switch and Claude Code plugin cache

### Bug fixes & infrastructure

- Fixed `tauri.conf.json` version mismatch (was 0.2.3, caused builds to output wrong filenames)
- Release page now opens via Tauri opener API instead of blocked `window.open`
- Consolidated release workflow constraint into CLAUDE.md
- Cleaned up all stale `--model` / `--auto` references across skill docs

---

## [v0.3.0] — 2026-04-13

### Changed

- **Delegate progressive reporting**: When running parallel agents, each completed agent's result is immediately reported to the user. A unified summary table is shown when all agents finish.
- **Delegate commands simplified**: Removed `--model`, `--auto`, and `group/member` support from `--to`. Only two forms remain: `/delegate <task>` (main model analyzes task and chooses strategy) and `/delegate --to <group> <task>` (explicit group target).
- **Task analysis phase added**: Without flags, the main model now analyzes the task before dispatching and chooses among three strategies — serial multi-agent (dependent subtasks), parallel multi-agent (independent subtasks), or serial single-agent (simple/tightly coupled tasks).
- **Skills page delete confirmation**: Replaced browser-native `window.confirm` with the project's `ConfirmDialog` modal for deleting task-route preferences.
- **i18n cleanup**: Removed duplicate `groupsEmpty` key in zh-CN bundle that caused CI build failures.

### Added

- **Parallel task splitting**: `/delegate` now supports analyzing a request, splitting into distinct subtasks, dispatching to respective agents in parallel, retrying failures, and producing a unified report.
- **Skills page refresh**: Redesigned Skills tab with improved layout, styling, and task-route preference management (add/edit/delete entries).
- **ConfirmDialog modal**: New `Delete preference` confirmation dialog for skills preferences.
- **i18n updates**: New labels for `deletePreferenceConfirmTitle` / `deletePreferenceConfirmBody` in both English and Chinese.

### Removed

- Removed `/delegate --model <member>`, `/delegate --auto`, and `--to <group>/<member>` syntax.
- Removed `delegate-auto` skill reference from docs.

---

## [v0.2.5] — 2026-04-13

### Changed

- **Delegate simplified**: Replaced five specialized delegate workers (`auto`, `haiku`, `inherit`, `opus`, `sonnet`) with generated agents from Skills page preferences.
- **Delegate SKILL.md rewritten**: Streamlined documentation, added task analysis phase, parallel dispatch strategy.
- **Task route preferences extended**: Added `delegate_model` field for storing preferred model per task kind.

### Added

- **Skills page refresh**: Redesigned Skills tab with improved layout and task-route preference management.
- **Migration 007**: `add_delegate_model` column to `task_route_preference` table.
- **ConfirmDialog modal**: Delete confirmation dialog for skills preferences.

### Removed

- Deleted `agents/octoswitch-delegate-{auto,haiku,inherit,opus,sonnet}-worker.md` — replaced by generated agents.

---

## [v0.2.4] — 2026-04-13

### Changed

- **Delegate simplified**: Replaced five specialized delegate workers with a single `octoswitch-delegate-default-worker`.
- **Delegate SKILL.md rewritten**: Streamlined documentation, removed redundant routing examples.
- **Task route preferences extended**: Added `delegate_model` field.

### Added

- **Skills page refresh**: Redesigned Skills tab.
- **Migration 007**: `add_delegate_model` column.
- **i18n updates**: New labels for delegate model and skills UI.

### Removed

- Deleted `agents/octoswitch-delegate-{auto,haiku,inherit,opus,sonnet}-worker.md`.

---

## [v0.2.3] — 2026-04-13

### Added

- **Parallel task splitting**: `/delegate` now supports analyzing requests and splitting into subtasks for parallel dispatch.
- **Task analysis phase**: Main model analyzes tasks and chooses execution strategy before dispatching.

### Changed

- **Delegate command surface**: Simplified to `/delegate <task>` and `/delegate --to <group> <task>`.
- **Agents use group names**: Agent `model` field set to group name for OctoSwitch gateway routing, enabling real-time member switching.
- **Auto-sync on CRUD**: Preference changes now automatically sync plugin files to both cc-switch and Claude Code cache.
- **DAO fixes**: `update_partial()` now respects `target_member` and `delegate_model` patch values instead of hardcoding to None.
- **Import SQL fix**: `import_config` now includes `delegate_agent_kind` and `delegate_model` columns.

### Fixed

- **Generated agents to Claude Code cache**: Added `patch_claude_code_plugin_cache()` to write generated agents to Claude Code's plugin cache (`~/.claude/plugins/cache/`), not just cc-switch.
- **Default worker removed**: Deleted stale `octoswitch-delegate-default-worker.md` to prevent wrong agent selection.

---

## [v0.2.2] — 2026-04-13

### Added

- **Real subagent delegation**: `/delegate` now launches actual Claude Code subagents via the Task tool.
- **Namespaced delegate agent ID**: Fixed agent namespace to `octoswitch:` prefix.

### Changed

- **Delegate model**: Switched from five specialized workers to a single default worker, then later to generated agents from preferences.
- **Route binding documentation**: Clarified limitations on route binding.

---

## [v0.2.1] — 2026-04-13

### Added

- **Offline detection**: Added offline detection for routing helper.
- **Skills page**: New Skills tab for plugin repo workflow.
- **Plugin marketplace**: Made plugin installable from repo URL.
- **Marketplace manifest**: Aligned with Claude schema.

### Fixed

- **Release action**: Restored working Tauri release workflow.

---

## [v0.2.0] — 2026-04-13

### Added

- **Plugin dist export pipeline**: Added build pipeline for distributable plugin artifacts.
- **Marketplace flow**: Moved plugin management to repo-root marketplace flow.
- **Claude Code routing roadmap**: Added design-stage routing entries.

### Changed

- **Version alignment**: Aligned app version to 0.2.0.

---

## [v0.1.0] — Initial Release

### Added

- **Tray controls and update checker**: System tray menu with app controls and automatic update checking.
- **Autostart and tray behavior**: Refined autostart and tray menu interactions.
- **Skills routing management**: Added skills-based routing management workflow.
- **Cached usage tokens**: Track cached read/write tokens in gateway metrics.
