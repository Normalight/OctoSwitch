# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**OctoSwitch** — a Tauri 2 desktop app that acts as a local model routing gateway. It proxies LLM API requests to upstream providers (OpenAI/Anthropic compatible) with model grouping, binding/routing, metrics collection, and security auditing.

## Tech Stack


| Layer           | Technology                                      |
| --------------- | ----------------------------------------------- |
| Frontend        | React 18 + TypeScript, plain CSS (no framework) |
| Desktop shell   | Tauri 2                                         |
| Gateway backend | Rust (axum + tokio)                             |
| Database        | SQLite (rusqlite, bundled)                      |
| Build tooling   | Vite 5                                          |
| Charting        | recharts                                        |


## Architecture

```
Frontend (React)  ←Tauri invoke→  Rust commands  →  Service layer  →  Repository traits  →  SQLite DAOs
                                                                  ↓
                                                        Gateway server (axum)
                                                                  ↓
                                                      Upstream providers (OpenAI/Anthropic)
```

### Key modules

**Rust backend (`src-tauri/src/`):**

- `commands/` — Tauri-invokable commands for CRUD on providers, bindings, model groups, security, metrics
- `domain/` — domain types and unified `AppError` enum (via `thiserror`)
- `gateway/` — Embedded axum HTTP server that proxies requests to upstream LLM providers
  - `gateway/router.rs` — routes incoming requests to the correct upstream
  - `gateway/forwarder.rs` — handles the actual HTTP forwarding (uses `routing_service`)
  - `gateway/protocol/` — OpenAI and Anthropic API adapters (normalize request/response formats)
  - `gateway/routes/` — includes a subagent route
  - `gateway/error.rs` — `ForwardRequestError` with `From<AppError>` impl
- `config/` — `AppConfig` struct (gateway_port, gateway_host, db_path, http_proxy)
- `database/` — DAOs for providers, model bindings, model groups, model group members; `init_schema` uses migration system
- `repository/` — `traits.rs` defines `ProviderRepo`/`ModelBindingRepo`/`ModelGroupRepo` traits; `sqlite/` contains SQL migrations
- `service/` — thin wrappers around DAOs, return `Result<_, AppError>` instead of `String`
- `services/` — business logic (health checks, circuit breaker, audit, metrics collector/aggregator, config import/export, HTTP client, model resolution, security)
- `state.rs` — shared application state (`db` + `config`) passed to Tauri commands and gateway

**Database migrations (`src-tauri/src/repository/sqlite/migrations/`):**

- `001_initial_schema.sql` — single comprehensive schema with all current tables
- `migrations.rs` — `INITIAL_SCHEMA` constant + `run_migrations` incremental runner
- `schema_version` table tracks current version; uses `INSERT OR IGNORE` for safe re-init

**Frontend (`src/`):**

- `App.tsx` — tab-based shell (providers / models / usage / settings)
- `pages/` — one component per tab
- `hooks/` — `useProviders`, `useModels`, `useModelGroups` — data fetching via Tauri invokes
- `lib/api/tauri.ts` — typed Tauri API client with zod `parseAsync` validation
- `api/schemas.ts` — zod schemas for all domain types
- `types/` — split domain types (`provider.ts`, `model_binding.ts`, `model_group.ts`, `metrics.ts`)
- `i18n/` — custom i18n system (en, zh-CN bundles, context-based)
- `theme/` — dark/light theme context

### Data model relationships

```
Provider 1──N ModelBinding N──M ModelGroup
                    │              │
              (via model_group_members join table)
```

- **Provider** — upstream LLM API endpoint (base_url, protocol, api_key_ref)
- **ModelBinding** — maps a local model name to an upstream model on a specific provider
- **ModelGroup** — alias groupings (e.g. "Sonnet") that can have multiple bindings and an active binding selection

## Important Conventions

- Tauri commands are invoked from the frontend via `invoke()` — the frontend API client in `src/lib/api/tauri.ts` centralizes all invokes with zod validation
- Rust types in `src-tauri/src/domain/` and `src-tauri/src/models/` should correspond to TypeScript types in `src/types/`
- The gateway runs as an embedded axum server inside the Tauri app, listening on a configurable port
- i18n uses a custom lightweight system (not react-i18next) — see `src/i18n/`
- Config import/export is JSON-based and includes all providers, bindings, and groups
- `AppError` (in `domain/error.rs`) is the unified error type — service layer returns `Result<_, AppError>`, commands convert to `String` for Tauri
- Repository traits (`repository/traits.rs`) define `*Repo` traits with `async_trait` — SQLite impl is via the DAO layer in `database/`
- When adding new database columns, create a new numbered migration file — never add columns directly to 001_initial_schema.sql
- Frontend schema imports: from `src/lib/api/tauri.ts` use `../../api/schemas`
- There is no test framework configured yet

## Strict Constraints

- **Forwarding code requires user review**: Any changes to `src-tauri/src/gateway/` (forwarder, router, protocol adapters, routes, error handling) MUST be presented to the user for review before committing. Do not auto-commit gateway/forwarding changes.
- **Python testing before commit**: All features and fixes must pass Python-based test scripts (e.g., `scripts/*.py`, `test_*.py`) before committing. These serve as the project's integration tests for gateway behavior, streaming, and routing. If no test exists for the changed area, write one first.
- **Version bump on release**: When creating a release tag (e.g., `vX.Y.Z`), ALL three version sources MUST be bumped to match:
  - `src-tauri/Cargo.toml` → `version = "X.Y.Z"`
  - `package.json` → `"version": "X.Y.Z"`
  - `.claude-plugin/plugin.json` → `"version": "X.Y.Z"`
  - Then commit, push, and create the tag. The tag version MUST match the version in all three files exactly.

## Future Features

- **Fine-grained request timeouts** (cc-switch model): Per-provider or global configurable timeouts with three dimensions — `streaming_first_byte_timeout` (wait for first SSE chunk), `streaming_idle_timeout` (silence between chunks), `non_streaming_timeout` (entire non-streaming request). Streaming timeouts are enforced via `tokio::time::timeout` wrapping `stream.next()` in a two-phase model. In cc-switch, these are gated behind `auto_failover_enabled` — timeouts only activate when failover is available. Reference: cc-switch `proxy/handler_context.rs` `StreamingTimeoutConfig`, `proxy/response_processor.rs` `create_logged_passthrough_stream()`.

