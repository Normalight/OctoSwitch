use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    config::app_config::repo_root_dir,
    domain::plugin_dist::PluginConfig,
    domain::local_skill::{LocalPluginStatus, LocalPluginSyncResult},
    service::plugin_dist_service::generated_delegate_agent_name,
};

#[derive(Debug, Deserialize)]
struct MarketplaceManifest {
    #[serde(default)]
    plugins: Vec<MarketplacePluginEntry>,
}

#[derive(Debug, Deserialize)]
struct MarketplacePluginEntry {
    name: String,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

fn collect_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return Ok(out);
    }

    fn walk(
        base: &Path,
        current: &Path,
        out: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(base)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read(&path).map_err(|e| e.to_string())?);
            }
        }
        Ok(())
    }

    walk(root, root, &mut out)?;
    Ok(out)
}

fn collect_plugin_source_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut out = BTreeMap::new();
    for relative_root in [".claude-plugin", "skills", "agents"] {
        let dir = root.join(relative_root);
        if !dir.exists() {
            continue;
        }
        let files = collect_files(&dir)?;
        for (relative, contents) in files {
            out.insert(format!("{relative_root}/{relative}"), contents);
        }
    }
    Ok(out)
}

fn find_installed_plugin_dir(plugins_root: &Path, plugin_name: &str) -> Option<PathBuf> {
    let direct = plugins_root.join(plugin_name);
    if direct.join(".claude-plugin").join("plugin.json").exists() {
        return Some(direct);
    }

    let entries = fs::read_dir(plugins_root).ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let manifest = path.join(".claude-plugin").join("plugin.json");
        if !manifest.exists() {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&manifest) else {
            continue;
        };
        if contents.contains(&format!("\"name\": \"{plugin_name}\"")) {
            return Some(path);
        }
    }

    None
}

fn resolve_repo_reference_to_local_path(marketplace_path: &Path, repo: &str) -> PathBuf {
    let repo = repo.trim();
    let marketplace_root = marketplace_path
        .parent()
        .and_then(|v| v.parent())
        .unwrap_or_else(|| Path::new("."));

    if repo.is_empty() {
        return repo_root_dir();
    }

    let repo_path = Path::new(repo);
    if repo_path.is_absolute() {
        return repo_path.to_path_buf();
    }

    if repo.starts_with("./") || repo.starts_with("../") || repo.contains('\\') {
        return marketplace_root.join(repo_path);
    }

    if repo.contains('/') {
        return repo_root_dir();
    }

    marketplace_root.join(repo)
}

fn resolve_marketplace_plugin_repo(
    marketplace_manifest_path: &str,
    plugin_name: &str,
) -> Result<(String, PathBuf), String> {
    let manifest_path = Path::new(marketplace_manifest_path);
    let contents = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read marketplace manifest {}: {e}", manifest_path.display()))?;
    let manifest: MarketplaceManifest = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse marketplace manifest {}: {e}", manifest_path.display()))?;

    let entry = manifest
        .plugins
        .into_iter()
        .find(|plugin| plugin.name.eq_ignore_ascii_case(plugin_name))
        .ok_or_else(|| format!("Plugin `{plugin_name}` not found in marketplace manifest"))?;

    let repo_ref = entry
        .repo
        .or(entry.source)
        .ok_or_else(|| {
            format!(
                "Plugin `{plugin_name}` in marketplace manifest must provide `repo` or `source`"
            )
        })?;

    let tracked_path = resolve_repo_reference_to_local_path(manifest_path, &repo_ref);
    Ok((repo_ref, tracked_path))
}

fn generated_agent_relative_path(task_kind: &str) -> Option<String> {
    generated_delegate_agent_name(task_kind).map(|name| format!("agents/generated/{name}.md"))
}

fn generated_agent_doc(
    namespace: &str,
    task_kind: &str,
    route: &str,
    agent_name: &str,
) -> String {
    format!(
        "---\nname: {agent_name}\ndescription: Execute OctoSwitch delegated `{task_kind}` tasks for route `{route}`.\nmodel: inherit\n---\n\nYou are the OctoSwitch delegated worker for task kind `{task_kind}`.\n\nYou are running in a fresh subagent launched by `/{namespace}:delegate`.\nTreat the route supplied by the controller as fixed task metadata.\n\nReturn only these sections:\n\n- `route confirmation`\n- `summary`\n- `files changed`\n- `commands run`\n- `test results`\n- `unresolved risks`\n\nThe `route confirmation` section must explicitly state:\n\n- requested route received from controller\n- preferred task kind: `{task_kind}`\n- preferred route from config: `{route}`\n- launched worker: `{namespace}:{agent_name}`\n- runtime model: `inherit`\n\nIf no files were changed, say so explicitly.\n"
    )
}

fn generated_agent_files(runtime_config: &PluginConfig) -> Result<(BTreeMap<String, Vec<u8>>, Vec<String>), String> {
    let mut files = BTreeMap::new();
    let mut agent_names = Vec::new();

    for (task_kind, route) in &runtime_config.task_routes {
        if !route.enabled {
            continue;
        }
        let Some(agent_name) = generated_delegate_agent_name(task_kind) else {
            return Err(format!("Failed to generate delegate agent name for task kind `{task_kind}`"));
        };
        let Some(relative_path) = generated_agent_relative_path(task_kind) else {
            return Err(format!("Failed to generate delegate agent path for task kind `{task_kind}`"));
        };
        let route_label = match &route.member {
            Some(member) if !member.trim().is_empty() => format!("{}/{}", route.group, member),
            _ => route.group.clone(),
        };
        files.insert(
            relative_path,
            generated_agent_doc(
                &runtime_config.namespace,
                task_kind,
                &route_label,
                &agent_name,
            )
            .into_bytes(),
        );
        agent_names.push(agent_name);
    }

    Ok((files, agent_names))
}

fn rendered_plugin_manifest(
    tracked_root: &Path,
    generated_agent_paths: &[String],
) -> Result<(Vec<u8>, usize), String> {
    let manifest_path = tracked_root.join(".claude-plugin").join("plugin.json");
    let contents = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read {}: {e}", manifest_path.display()))?;
    let mut manifest: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse {}: {e}", manifest_path.display()))?;

    let Some(obj) = manifest.as_object_mut() else {
        return Err(format!("Plugin manifest {} is not a JSON object", manifest_path.display()));
    };

    let mut agent_paths = obj
        .get("agents")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for relative in generated_agent_paths {
        let relative = format!("./{relative}");
        if !agent_paths.iter().any(|item| item == &relative) {
            agent_paths.push(relative);
        }
    }

    obj.insert(
        "agents".to_string(),
        Value::Array(agent_paths.iter().cloned().map(Value::String).collect()),
    );

    let registered_agent_count = agent_paths.len();
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    Ok((bytes, registered_agent_count))
}

fn expected_plugin_files(
    tracked_root: &Path,
    runtime_config: &PluginConfig,
) -> Result<(BTreeMap<String, Vec<u8>>, Vec<String>, usize), String> {
    let mut files = collect_plugin_source_files(tracked_root)?;
    let (generated_files, generated_agents) = generated_agent_files(runtime_config)?;
    let generated_paths = generated_files.keys().cloned().collect::<Vec<_>>();
    let (manifest_bytes, registered_agent_count) =
        rendered_plugin_manifest(tracked_root, &generated_paths)?;

    files.insert(".claude-plugin/plugin.json".to_string(), manifest_bytes);
    files.insert(
        ".claude-plugin/plugin.config.json".to_string(),
        serde_json::to_vec_pretty(runtime_config).map_err(|e| e.to_string())?,
    );

    for (relative, contents) in generated_files {
        files.insert(relative, contents);
    }

    Ok((files, generated_agents, registered_agent_count))
}

fn sync_directories(
    expected_files: &BTreeMap<String, Vec<u8>>,
    installed_root: &Path,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), String> {
    let installed_files = collect_files(installed_root)?;

    fs::create_dir_all(installed_root).map_err(|e| e.to_string())?;

    let mut copied_files = Vec::new();
    for (relative, contents) in expected_files {
        let dst = installed_root.join(relative);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&dst, contents).map_err(|e| e.to_string())?;
        copied_files.push(relative.clone());
    }

    let mut removed_files = Vec::new();
    let preserved_files = Vec::new();
    for relative in installed_files.keys() {
        if expected_files.contains_key(relative) {
            continue;
        }
        let path = installed_root.join(relative);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            removed_files.push(relative.clone());
        }
    }

    fn remove_empty_dirs(root: &Path, current: &Path) -> Result<(), String> {
        for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                remove_empty_dirs(root, &path)?;
            }
        }
        if current != root && fs::read_dir(current).map_err(|e| e.to_string())?.next().is_none() {
            fs::remove_dir(current).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    remove_empty_dirs(installed_root, installed_root)?;
    Ok((copied_files, removed_files, preserved_files))
}

pub fn inspect_cc_switch_plugin_status(
    marketplace_manifest_path: &str,
    plugins_root_path: &str,
    plugin_name: &str,
    runtime_config: &PluginConfig,
) -> Result<LocalPluginStatus, String> {
    let (marketplace_repo, tracked_root_buf) =
        resolve_marketplace_plugin_repo(marketplace_manifest_path, plugin_name)?;
    let tracked_root = tracked_root_buf.as_path();
    let plugins_root = Path::new(plugins_root_path);
    let installed_root = find_installed_plugin_dir(plugins_root, plugin_name)
        .unwrap_or_else(|| plugins_root.join(plugin_name));

    let (expected_files, generated_agents, registered_agent_count) =
        expected_plugin_files(tracked_root, runtime_config)?;
    let installed_files = collect_files(&installed_root)?;

    let missing_files = expected_files
        .keys()
        .filter(|k| !installed_files.contains_key(*k))
        .cloned()
        .collect::<Vec<_>>();

    let changed_files = expected_files
        .iter()
        .filter_map(|(k, v)| match installed_files.get(k) {
            Some(installed) if installed != v => Some(k.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    Ok(LocalPluginStatus {
        marketplace_path: marketplace_manifest_path.to_string(),
        marketplace_repo,
        tracked_path: tracked_root.to_string_lossy().to_string(),
        installed_path: installed_root.to_string_lossy().to_string(),
        tracked_exists: tracked_root.exists(),
        installed_exists: installed_root.exists(),
        up_to_date: tracked_root.exists()
            && installed_root.exists()
            && missing_files.is_empty()
            && changed_files.is_empty(),
        tracked_file_count: expected_files.len(),
        installed_file_count: installed_files.len(),
        registered_agent_count,
        generated_agents,
        missing_files,
        changed_files,
    })
}

pub fn sync_cc_switch_plugin_from_marketplace(
    marketplace_manifest_path: &str,
    plugins_root_path: &str,
    plugin_name: &str,
    runtime_config: &PluginConfig,
) -> Result<LocalPluginSyncResult, String> {
    let (_marketplace_repo, tracked_root_buf) =
        resolve_marketplace_plugin_repo(marketplace_manifest_path, plugin_name)?;
    let tracked_root = tracked_root_buf.as_path();
    let plugins_root = Path::new(plugins_root_path);
    let installed_root = find_installed_plugin_dir(plugins_root, plugin_name)
        .unwrap_or_else(|| plugins_root.join(plugin_name));

    if !tracked_root.exists() {
        return Err(format!(
            "Tracked plugin repo not found: {}",
            tracked_root.display()
        ));
    }

    let (expected_files, _generated_agents, _registered_agent_count) =
        expected_plugin_files(tracked_root, runtime_config)?;
    fs::create_dir_all(plugins_root).map_err(|e| e.to_string())?;
    let (copied_files, removed_files, preserved_files) =
        sync_directories(&expected_files, &installed_root)?;
    let status = inspect_cc_switch_plugin_status(
        marketplace_manifest_path,
        plugins_root_path,
        plugin_name,
        runtime_config,
    )?;

    Ok(LocalPluginSyncResult {
        status,
        copied_files,
        removed_files,
        preserved_files,
    })
}
