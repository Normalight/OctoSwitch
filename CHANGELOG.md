# Changelog

## [v0.4.2] — 2026-05-04

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
