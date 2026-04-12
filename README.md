# OctoSwitch

Desktop app and local **LLM API gateway** for personal use. Point tools like [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) at one stable URL, manage upstream providers in the UI, and switch models without restarting the client.

面向个人使用的桌面应用与本地 **大模型 API 网关**：客户端固定指向本机网关，在界面中管理供应商、分组与路由，用**分组别名**统一对外模型名，并在组内切换活动上游模型。

---

## Interface

### Providers（供应商）

Configure upstream connections by API format (OpenAI-compatible, Anthropic-style, GitHub Copilot, etc.), attach model bindings, reorder cards, and use **Copilot reverse proxy** or **Add provider** from the toolbar.

<p align="center">
  <img src="assets/readme/overview.png" alt="OctoSwitch — Providers tab" width="780" />
  <br /><em>供应商：API 格式、已绑定模型与快捷操作</em>
</p>

### Groups（分组）

Group aliases are the **model names your clients send** (e.g. `Sonnet`, `Opus`). Each card shows the active provider binding; the same binding can appear in multiple groups, with a per-group **active target model**.

<p align="center">
  <img src="assets/readme/groups.png" alt="OctoSwitch — Groups tab" width="780" />
  <br /><em>分组列表：别名 ↔ 活动上游模型</em>
</p>

### Group editor（编辑模型分组）

Open a group to add members from different providers, mark one member as **active**, and keep the client-facing alias stable while you switch backends.

<p align="center">
  <img src="assets/readme/models.png" alt="OctoSwitch — Edit model group" width="780" />
  <br /><em>分组内多成员、设活动模型、与别名校验提示</em>
</p>

### Settings → Data（设置 · 数据）

Export or import JSON config, **one-click import from cc-switch** (reads the local cc-switch database; supports Claude / Codex / Gemini-style entries among others), and local-only SQLite storage with privacy notes on secrets.

<p align="center">
  <img src="assets/readme/settings.png" alt="OctoSwitch — Settings Data tab" width="780" />
  <br /><em>配置导入导出与本地数据说明</em>
</p>

### Import from cc-switch（从 cc-switch 一键导入）

[cc-switch](https://github.com/farion1231/cc-switch) keeps Claude / Codex / Gemini-style provider entries in a local SQLite database. On the same machine, open **Settings → Data** and use **One-click import** (一键导入): OctoSwitch reads those rows and creates matching providers and model bindings. You can then refine groups and upstream model lists in OctoSwitch without retyping endpoints.

若本机已安装并配置 cc-switch，OctoSwitch 可在 **设置 → 数据** 中通过 **一键导入** 读取 cc-switch 本地数据库中的供应商与模型配置（支持 Claude、Codex、Gemini 等类型），导入后在分组与供应商页继续调整即可。

<p align="center">
  <img src="assets/readme/cc-switch.png" alt="OctoSwitch — Import from cc-switch (Settings → Data)" width="780" />
  <br /><em>设置 → 数据：从 cc-switch 导入配置（一键导入）</em>
</p>

---

## Features

- **Multi-provider:** OpenAI-compatible APIs, Anthropic-style routes, GitHub Copilot, and more—per-card format and bindings.
- **Groups & aliases:** One stable **group alias** for clients; multiple upstream members per group with an **active** model for quick switching.
- **Upstream model list:** On model bindings, **fetch model list** (`GET /v1/models` or Copilot’s discovery where supported); the UI reports counts and can fill the upstream field.
- **Gateway discovery:** Local gateway exposes **`GET /v1/models`** for tools and scripts (exact shape depends on gateway options, e.g. group-only vs group/member listing).
- **cc-switch:** Import providers and bindings from the **cc-switch** SQLite DB on the same machine (**Settings → Data → One-click import**).
- **Usage & resilience:** Usage metrics, health checks, config backup/restore.
- **i18n & desktop UX:** English / Chinese UI, light & dark theme, tray menu, optional autostart.

### Import from cc-switch

1. Install and configure [cc-switch](https://github.com/farion1231/cc-switch) as usual (its local database must exist).
2. In OctoSwitch: **Settings → Data → One-click import** (一键导入).
3. Review the import report; the Models / Groups views refresh so you can adjust groups or fetch upstream lists without retyping.

This is a **one-shot migration**, not live sync—run import again after you change cc-switch.

**中文：** 与 cc-switch 共用本机配置：在 **设置 → 数据** 一键读取 cc-switch 数据库中的供应商（含 Claude、Codex、Gemini 等类型），导入后继续用分组与「获取模型列表」微调。

---

## Requirements

- Node.js 18+
- Rust (stable) and [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
npm install
npm run tauri:dev
```

## Build

```bash
npm run tauri:build
```

Other scripts: `npm run dev` (frontend only), `npm run build`, `npm run lint`, `npm run preview`.

Package name: `octoswitch`. App data (config, database, logs) lives under the OS local data directory in an **OctoSwitch** folder.

## Stack

React, TypeScript, Vite · Tauri 2 · Rust (axum, SQLite)

## Security

The embedded HTTP gateway is meant for **local** use. It does not implement its own API-key auth on proxy routes; avoid exposing the listen address beyond your machine. Keys and tokens are stored locally; treat exported config like secrets.

## Acknowledgments

Inspired by [cc-switch](https://github.com/farion1231/cc-switch) and [copilot-api](https://github.com/caozhiyuan/copilot-api).

## Copilot disclaimer

> [!WARNING]
> GitHub Copilot support in OctoSwitch relies on **unofficial / reverse-engineered** access to Copilot services (similar in spirit to community Copilot proxies). It is **not supported by GitHub** and **may break unexpectedly**. **Use at your own risk.** Sign-in and request flows may send account-related data or client telemetry to GitHub as part of Copilot usage. Using many Copilot accounts from the same environment can increase risk; prefer isolation (for example separate OS profiles or containers) when in doubt.

> [!WARNING]
> **GitHub Security Notice:**  
> Excessive automated or scripted use of Copilot (including rapid or bulk requests, such as via automated tools) may trigger GitHub's abuse-detection systems.  
> You may receive a warning from GitHub Security, and further anomalous activity could result in temporary suspension of your Copilot access.
>
> GitHub prohibits use of their servers for excessive automated bulk activity or any activity that places undue burden on their infrastructure.
>
> Please review:
>
> - [GitHub Acceptable Use Policies](https://docs.github.com/site-policy/acceptable-use-policies/github-acceptable-use-policies#4-spam-and-inauthentic-activity-on-github)
> - [GitHub Copilot Terms](https://docs.github.com/site-policy/github-terms/github-terms-for-additional-products-and-features#github-copilot)
>
> Use Copilot-related features in OctoSwitch **responsibly** to avoid account restrictions.

**中文简要说明：** Copilot 相关能力基于非官方接口，不受 GitHub 官方支持，可能随时失效；请求与登录过程仍可能向 GitHub 提交与账号或客户端相关的数据。过度自动化、高频或批量调用可能触发滥用检测并导致警告或暂时限制。**请自行承担风险**，并务必阅读上文 GitHub 政策链接。

## License

[MIT](LICENSE)
