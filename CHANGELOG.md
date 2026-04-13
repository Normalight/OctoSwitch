# Changelog

## [v0.2.5] — 2026-04-13

### Changed

- **Delegate commands simplified**: Removed `--model`, `--auto`, and `group/member` support from `--to`. Only two forms remain: `/delegate <task>` (main model analyzes task and chooses strategy) and `/delegate --to <group> <task>` (explicit group target).
- **Task analysis phase added**: Without flags, the main model now analyzes the task before dispatching and chooses among three strategies — serial multi-agent (dependent subtasks), parallel multi-agent (independent subtasks), or serial single-agent (simple/tightly coupled tasks). The plan is presented for user confirmation before any agents are launched.
- **All routing through group names only**: Agents use group names (e.g. `Sonnet`, `Haiku`) as their `model` field, so requests always go through the OctoSwitch gateway where active members can be switched in real time. No more `target_member` or `group/member` paths in delegate.

### Removed

- Removed `/delegate --model <member>`, `/delegate --auto`, and `--to <group>/<member>` syntax from SKILL.md.

---

## [v0.2.4] — 2026-04-13

### Changed

- **Delegate simplified**: Replaced five specialized delegate workers (`auto`, `haiku`, `inherit`, `opus`, `sonnet`) with a single `octoswitch-delegate-default-worker`. All explicit and `--auto` routing now falls back to this worker unless a generated agent is configured via Skills page preferences.
- **Delegate SKILL.md rewritten**: Streamlined documentation, removed redundant routing examples, clarified `--auto` mode semantics (classify → consult `/task-route` preferences → choose route → launch matching subagent).
- **Task route preferences extended**: Added `delegate_model` field for storing preferred model per task kind, used by automatic delegation.

### Added

- **Skills page refresh**: Redesigned Skills tab with improved layout, styling, and task-route preference management (add/edit/delete entries).
- **Migration 007**: `add_delegate_model` column to `task_route_preference` table.
- **i18n updates**: New labels for delegate model, skills management, and task-route UI in both English and Chinese.

### Removed

- Deleted `agents/octoswitch-delegate-{auto,haiku,inherit,opus,sonnet}-worker.md` — replaced by single default worker.

---

## [v0.2.3] — Previous release

- Initial plugin marketplace support
- Claude Code routing skills
- Provider and model group management
