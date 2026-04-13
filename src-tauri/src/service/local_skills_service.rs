use std::{fs, path::Path};

use crate::domain::local_skill::{LocalSkillEntry, LocalSkillsStatus};

const PROJECT_SKILLS: &[&str] = &[
    "delegate",
    "install",
    "route-activate",
    "show-routing",
    "task-route",
];

fn is_project_skill(name: &str) -> bool {
    PROJECT_SKILLS
        .iter()
        .any(|skill| skill.eq_ignore_ascii_case(name))
}

fn list_skills(path: &str) -> Vec<LocalSkillEntry> {
    let root = Path::new(path);
    let Ok(entries) = fs::read_dir(root) else {
        return vec![];
    };

    let mut skills = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().to_string();
            if !is_project_skill(&name) {
                return None;
            }
            let skill_md = path.join("SKILL.md");
            Some(LocalSkillEntry {
                name,
                path: path.to_string_lossy().to_string(),
                has_skill_md: skill_md.exists(),
            })
        })
        .collect::<Vec<_>>();

    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    skills
}

pub fn inspect_skills_paths(source_path: &str, installed_path: &str) -> LocalSkillsStatus {
    LocalSkillsStatus {
        source_path: source_path.to_string(),
        installed_path: installed_path.to_string(),
        source_exists: Path::new(source_path).exists(),
        installed_exists: Path::new(installed_path).exists(),
        source_skills: list_skills(source_path),
        installed_skills: list_skills(installed_path),
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn install_repo_skills_to_path(repo_skills_path: &str, target_path: &str) -> Result<(), String> {
    let source_root = Path::new(repo_skills_path);
    if !source_root.exists() {
        return Err(format!("Skills source folder not found: {}", source_root.display()));
    }

    fs::create_dir_all(target_path).map_err(|e| e.to_string())?;

    let entries = fs::read_dir(source_root).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().map(|v| v.to_string_lossy().to_string()) else {
            continue;
        };
        if !is_project_skill(&name) {
            continue;
        }
        if !path.join("SKILL.md").exists() {
            continue;
        }
        let target = Path::new(target_path).join(entry.file_name());
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        }
        copy_dir_recursive(&path, &target)?;
    }

    Ok(())
}
