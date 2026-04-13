# Changelog

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
