use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMemberStatus {
    pub binding_id: String,
    pub name: String,
    pub provider_id: String,
    pub upstream_model_name: String,
    pub enabled: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingGroupStatus {
    pub group_id: String,
    pub alias: String,
    pub enabled: bool,
    pub active_member: Option<String>,
    pub members: Vec<RoutingMemberStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStatus {
    pub allow_group_member_model_path: bool,
    pub groups: Vec<RoutingGroupStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActiveMemberRequest {
    pub member: String,
}
