export interface RoutingMemberStatus {
  binding_id: string;
  name: string;
  provider_id: string;
  upstream_model_name: string;
  enabled: boolean;
  active: boolean;
}

export interface RoutingGroupStatus {
  group_id: string;
  alias: string;
  enabled: boolean;
  active_member: string | null;
  members: RoutingMemberStatus[];
}

export interface RoutingStatus {
  allow_group_member_model_path: boolean;
  groups: RoutingGroupStatus[];
}
