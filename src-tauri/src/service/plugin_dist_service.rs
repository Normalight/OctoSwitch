use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use serde_json::json;

use crate::{
    config::app_config::{repo_root_skills_dir, GatewayConfig},
    database::task_route_preference_dao,
    domain::plugin_dist::{PluginConfig, PluginDistBuildResult, PluginTaskRouteConfig},
};

const COMMAND_SKILLS: &[&str] = &["delegate", "show-routing", "route-activate", "task-route"];

fn plugin_root(config: &GatewayConfig) -> PathBuf {
    PathBuf::from(&config.plugin_dist_path).join(&config.plugin_namespace)
}

fn marketplace_root(config: &GatewayConfig) -> PathBuf {
    PathBuf::from(&config.plugin_dist_path).join("marketplace")
}

fn default_group(config: &GatewayConfig) -> String {
    if config
        .plugin_namespace
        .eq_ignore_ascii_case("octoswitch")
    {
        "Sonnet".to_string()
    } else {
        config.plugin_namespace.clone()
    }
}

pub fn task_kind_agent_slug(task_kind: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in task_kind.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if !last_dash {
            last_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(next) = next {
            slug.push(next);
        }
    }

    slug.trim_matches('-').to_string()
}

pub fn generated_delegate_agent_name(task_kind: &str) -> Option<String> {
    let slug = task_kind_agent_slug(task_kind);
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| format!("Failed to clear {}: {e}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|e| format!("Failed to create {}: {e}", path.display()))
}

fn write_file(path: &Path, content: &str, files: &mut Vec<String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    files.push(path.to_string_lossy().to_string());
    Ok(())
}

fn load_skill_doc(name: &str) -> Result<String, String> {
    let path = repo_root_skills_dir().join(name).join("SKILL.md");
    fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

fn command_doc(namespace: &str, name: &str, body: &str) -> String {
    format!(
        "# /{namespace}:{name}\n\nExported from the OctoSwitch project-local skill `{name}`.\n\nCompatibility alias: `/{name}`.\n\n---\n\n{body}"
    )
}

fn shared_skill_doc(namespace: &str) -> String {
    format!(
        "# OctoSwitch Routing\n\nUse this shared skill with the `{namespace}` plugin commands.\n\n## Available commands\n\n- `/{namespace}:show-routing`\n- `/{namespace}:route-activate`\n- `/{namespace}:delegate`\n- `/{namespace}:task-route`\n\n## Routing model\n\n- `<group>` uses the group's current active member\n- `<group>/<member>` targets one explicit model route for a single task\n\n## Control plane API\n\n- `GET /healthz`\n- `GET /v1/routing/status`\n- `GET /v1/routing/groups/:alias/members`\n- `POST /v1/routing/groups/:alias/active-member`\n"
    )
}

fn agent_doc(namespace: &str) -> String {
    format!(
        "# OctoSwitch Executor\n\nYou are an execution-focused worker launched through the `{namespace}` plugin.\n\nReturn only:\n\n- summary\n- files changed\n- commands run\n- test results\n- unresolved risks\n"
    )
}

pub fn get_runtime_plugin_config(
    config: &GatewayConfig,
    conn: &Connection,
) -> Result<PluginConfig, String> {
    let preferences = task_route_preference_dao::list(conn)?;
    let mut task_routes = BTreeMap::new();

    for preference in preferences {
        let task_kind = preference.task_kind.clone();
        task_routes.insert(
            task_kind.clone(),
            PluginTaskRouteConfig {
                group: preference.target_group,
                member: None,
                delegate_model: None,
                delegate_agent_name: generated_delegate_agent_name(&task_kind),
                prompt_template: preference.prompt_template,
                enabled: preference.is_enabled,
            },
        );
    }

    Ok(PluginConfig {
        octoswitch_base_url: format!("http://{}:{}", config.host, config.port),
        namespace: config.plugin_namespace.clone(),
        default_group: default_group(config),
        task_routes,
        result_format: vec![
            "summary".to_string(),
            "files changed".to_string(),
            "commands run".to_string(),
            "test results".to_string(),
            "unresolved risks".to_string(),
        ],
    })
}

pub fn build_plugin_dist(
    config: &GatewayConfig,
    conn: &Connection,
) -> Result<PluginDistBuildResult, String> {
    let root = plugin_root(config);
    reset_dir(&root)?;

    let plugin_name = config.plugin_namespace.trim();
    if plugin_name.is_empty() {
        return Err("plugin namespace cannot be empty".to_string());
    }

    let mut files = Vec::new();
    let mut commands = Vec::new();

    for name in COMMAND_SKILLS {
        let body = load_skill_doc(name)?;
        let relative = format!("commands/{name}.md");
        write_file(
            &root.join(&relative),
            &command_doc(plugin_name, name, &body),
            &mut files,
        )?;
        commands.push(relative);
    }

    let shared_skill_relative = "skills/octoswitch-routing/SKILL.md".to_string();
    write_file(
        &root.join(&shared_skill_relative),
        &shared_skill_doc(plugin_name),
        &mut files,
    )?;

    let agent_relative = "agents/octoswitch-executor.md".to_string();
    write_file(
        &root.join(&agent_relative),
        &agent_doc(plugin_name),
        &mut files,
    )?;

    let manifest = json!({
        "name": plugin_name,
        "version": "0.1.0",
        "description": "Claude routing plugin for OctoSwitch local gateway.",
        "commands": commands,
        "skills": [shared_skill_relative],
        "agents": [agent_relative]
    });

    write_file(
        &root.join(".claude-plugin").join("plugin.json"),
        &serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
        &mut files,
    )?;

    let plugin_config = get_runtime_plugin_config(config, conn)?;
    write_file(
        &root.join(".claude-plugin").join("plugin.config.json"),
        &serde_json::to_string_pretty(&plugin_config).map_err(|e| e.to_string())?,
        &mut files,
    )?;

    Ok(PluginDistBuildResult {
        output_path: root.to_string_lossy().to_string(),
        files,
        plugin_config: Some(plugin_config),
    })
}

pub fn build_marketplace_dist(config: &GatewayConfig) -> Result<PluginDistBuildResult, String> {
    let root = marketplace_root(config);
    reset_dir(&root)?;

    let mut files = Vec::new();
    let manifest = json!({
        "plugins": [
            {
                "name": config.plugin_namespace,
                "repo": ".",
                "version": "0.1.0",
                "description": "Claude Code routing plugin for OctoSwitch local gateway."
            }
        ]
    });

    write_file(
        &root.join(".claude-plugin").join("marketplace.json"),
        &serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
        &mut files,
    )?;

    Ok(PluginDistBuildResult {
        output_path: root.to_string_lossy().to_string(),
        files,
        plugin_config: None,
    })
}
