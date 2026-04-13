export interface TaskRoutePreference {
  id: string;
  task_kind: string;
  target_group: string;
  target_member: string | null;
  delegate_agent_kind: "auto" | "inherit" | "sonnet" | "opus" | "haiku";
  prompt_template: string | null;
  is_enabled: boolean;
  sort_order: number;
}
