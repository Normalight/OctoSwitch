export interface LocalPluginStatus {
  marketplace_path: string;
  marketplace_repo: string;
  tracked_path: string;
  installed_path: string;
  tracked_exists: boolean;
  installed_exists: boolean;
  up_to_date: boolean;
  tracked_file_count: number;
  installed_file_count: number;
  registered_agent_count: number;
  generated_agents: string[];
  missing_files: string[];
  changed_files: string[];
}

export interface LocalPluginSyncResult {
  status: LocalPluginStatus;
  copied_files: string[];
  removed_files: string[];
  preserved_files: string[];
}

export interface CcSwitchDeeplink {
  url: string;
  resource_type: string;
  description: string;
}

export interface CcSwitchDeeplinkResult {
  provider_link: CcSwitchDeeplink | null;
  skill_link: CcSwitchDeeplink | null;
}
