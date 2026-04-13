use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSkillEntry {
    pub name: String,
    pub path: String,
    pub has_skill_md: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSkillsStatus {
    pub source_path: String,
    pub installed_path: String,
    pub source_exists: bool,
    pub installed_exists: bool,
    pub source_skills: Vec<LocalSkillEntry>,
    pub installed_skills: Vec<LocalSkillEntry>,
}
