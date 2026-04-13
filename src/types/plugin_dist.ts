export interface PluginTaskRouteConfig {
  group: string;
  member?: string | null;
  prompt_template?: string | null;
  enabled: boolean;
}

export interface PluginConfig {
  octoswitch_base_url: string;
  namespace: string;
  default_group: string;
  task_routes: Record<string, PluginTaskRouteConfig>;
  result_format: string[];
}

export interface PluginDistBuildResult {
  output_path: string;
  files: string[];
  plugin_config?: PluginConfig | null;
}
