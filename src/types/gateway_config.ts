export type LogLevel = "error" | "warn" | "info" | "debug" | "trace" | "off";

export const LOG_LEVELS: LogLevel[] = ["error", "warn", "info", "debug", "trace", "off"];

export interface GatewayHealthStatus {
  is_running: boolean;
  host: string;
  port: number;
  error: string | null;
}

export interface GatewayConfig {
  host: string;
  port: number;
  close_to_tray: boolean;
  /** 关闭到托盘时是否销毁主窗口（轻量模式，省内存、恢复较慢）；false 则仅 hide，恢复快 */
  light_tray_mode: boolean;
  /** 允许 `分组别名/绑定路由名` 作为 model，且 GET /v1/models 列出成员；false 时仅分组别名 */
  allow_group_member_model_path: boolean;
  auto_start: boolean;
  /** 仅当由开机自启动拉起且本项开启时，启动后不显示主窗口（仅托盘） */
  silent_autostart: boolean;
  log_level: LogLevel;
  debug_mode: boolean;
  skills_enabled: boolean;
  plugin_enabled: boolean;
  plugin_namespace: string;
  plugin_dist_path: string;
  marketplace_enabled: boolean;
  skills_source_path: string;
  claude_skills_path: string;
}
