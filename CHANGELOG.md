# Changelog

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
