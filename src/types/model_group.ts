// src/types/model_group.ts
export interface ModelGroup {
  id: string;
  alias: string;
  active_binding_id: string | null;
  is_enabled: boolean;
  sort_order: number;
}
