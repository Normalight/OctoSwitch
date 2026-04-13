use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    config::app_config::repo_root_dir,
    domain::local_skill::{LocalPluginStatus, LocalPluginSyncResult},
};

#[derive(Debug, Deserialize)]
struct MarketplaceManifest {
    #[serde(default)]
    plugins: Vec<MarketplacePluginEntry>,
}

#[derive(Debug, Deserialize)]
struct MarketplacePluginEntry {
    name: String,
    repo: String,
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
    for relative_root in [".claude-plugin", "skills"] {
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

    let tracked_path = resolve_repo_reference_to_local_path(manifest_path, &entry.repo);
    Ok((entry.repo, tracked_path))
}

fn sync_directories(
    tracked_root: &Path,
    installed_root: &Path,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), String> {
    let tracked_files = collect_plugin_source_files(tracked_root)?;
    let installed_files = collect_files(installed_root)?;
    let preserve_files = [".claude-plugin/plugin.config.json"];

    fs::create_dir_all(installed_root).map_err(|e| e.to_string())?;

    let mut copied_files = Vec::new();
    for relative in tracked_files.keys() {
        let src = tracked_root.join(relative);
        let dst = installed_root.join(relative);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        copied_files.push(relative.clone());
    }

    let mut removed_files = Vec::new();
    let mut preserved_files = Vec::new();
    for relative in installed_files.keys() {
        if tracked_files.contains_key(relative) {
            continue;
        }
        if preserve_files.iter().any(|keep| keep == relative) {
            preserved_files.push(relative.clone());
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
) -> Result<LocalPluginStatus, String> {
    let (marketplace_repo, tracked_root_buf) =
        resolve_marketplace_plugin_repo(marketplace_manifest_path, plugin_name)?;
    let tracked_root = tracked_root_buf.as_path();
    let plugins_root = Path::new(plugins_root_path);
    let installed_root = find_installed_plugin_dir(plugins_root, plugin_name)
        .unwrap_or_else(|| plugins_root.join(plugin_name));

    let tracked_files = collect_plugin_source_files(tracked_root)?;
    let installed_files = collect_files(&installed_root)?;

    let missing_files = tracked_files
        .keys()
        .filter(|k| !installed_files.contains_key(*k))
        .cloned()
        .collect::<Vec<_>>();

    let changed_files = tracked_files
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
        tracked_file_count: tracked_files.len(),
        installed_file_count: installed_files.len(),
        missing_files,
        changed_files,
    })
}

pub fn sync_cc_switch_plugin_from_marketplace(
    marketplace_manifest_path: &str,
    plugins_root_path: &str,
    plugin_name: &str,
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

    fs::create_dir_all(plugins_root).map_err(|e| e.to_string())?;
    let (copied_files, removed_files, preserved_files) =
        sync_directories(tracked_root, &installed_root)?;
    let status = inspect_cc_switch_plugin_status(
        marketplace_manifest_path,
        plugins_root_path,
        plugin_name,
    )?;

    Ok(LocalPluginSyncResult {
        status,
        copied_files,
        removed_files,
        preserved_files,
    })
}
