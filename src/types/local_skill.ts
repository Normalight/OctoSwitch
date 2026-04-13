export interface LocalSkillEntry {
  name: string;
  path: string;
  has_skill_md: boolean;
}

export interface LocalSkillsStatus {
  source_path: string;
  installed_path: string;
  source_exists: boolean;
  installed_exists: boolean;
  source_skills: LocalSkillEntry[];
  installed_skills: LocalSkillEntry[];
}
