use rusqlite::Connection;

use crate::config::app_config::load_gateway_config;
use crate::database::{model_binding_dao, model_group_dao, model_group_member_dao};
use crate::domain::error::AppError;
use crate::domain::model_binding::ModelBinding;
use crate::domain::model_spec_subcode::{
    DUPLICATE_ROUTING_NAME_IN_GROUP, GROUP_MEMBER_EMPTY_PART, MEMBER_PATH_DISABLED,
    MODEL_SPEC_EMPTY, ROUTING_NAME_CONTAINS_SLASH,
};
use crate::domain::routing::{RoutingGroupStatus, RoutingMemberStatus, RoutingStatus};
pub use crate::service::provider_service::get_provider;

/// 解析客户端 `model`：`分组别名`（走当前生效绑定）；若 `allow_member_path` 为真则还支持 `分组别名/绑定路由名`。
pub fn resolve_model_binding(
    conn: &Connection,
    model_spec: &str,
    allow_member_path: bool,
) -> Result<ModelBinding, AppError> {
    let spec = model_spec.trim();
    if spec.is_empty() {
        return Err(AppError::InvalidModelSpec {
            subcode: MODEL_SPEC_EMPTY,
            message: "Model name cannot be empty.".into(),
        });
    }

    if let Some((g_raw, b_raw)) = spec.split_once('/') {
        if !allow_member_path {
            return Err(AppError::InvalidModelSpec {
                subcode: MEMBER_PATH_DISABLED,
                message: "Group/member model paths are disabled. Use only the group alias as `model`, or enable \"Allow group/member routing names\" in Settings → Gateway.".into(),
            });
        }
        let group_key = g_raw.trim();
        let binding_key = b_raw.trim();
        if group_key.is_empty() || binding_key.is_empty() {
            return Err(AppError::InvalidModelSpec {
                subcode: GROUP_MEMBER_EMPTY_PART,
                message: "When using `groupAlias/routingName`, both the group alias and the binding routing name must be non-empty.".into(),
            });
        }
        if binding_key.contains('/') {
            return Err(AppError::InvalidModelSpec {
                subcode: ROUTING_NAME_CONTAINS_SLASH,
                message: "Binding routing name cannot contain '/'. Use exactly one slash: `groupAlias/routingName`.".into(),
            });
        }

        let g = model_group_dao::get_by_alias_ci(conn, group_key)
            .map_err(AppError::Internal)?
            .ok_or_else(|| AppError::ModelNotBound {
                model: spec.to_string(),
            })?;
        if !g.is_enabled {
            return Err(AppError::ModelNotBound {
                model: spec.to_string(),
            });
        }

        let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &g.id)
            .map_err(AppError::Internal)?;
        let mut matched: Option<ModelBinding> = None;
        for bid in binding_ids {
            let b = model_binding_dao::get_by_id(conn, &bid)
                .map_err(AppError::Internal)?
                .ok_or_else(|| AppError::Internal("成员绑定记录缺失".into()))?;
            if b.model_name == binding_key {
                if matched.is_some() {
                    return Err(AppError::InvalidModelSpec {
                        subcode: DUPLICATE_ROUTING_NAME_IN_GROUP,
                        message: format!(
                            "Group '{}' has more than one binding with routing name '{}'. Resolve the duplicate in the app.",
                            g.alias, binding_key
                        ),
                    });
                }
                matched = Some(b);
            }
        }
        let b = matched.ok_or_else(|| AppError::ModelNotBound {
            model: spec.to_string(),
        })?;
        if !b.is_enabled {
            return Err(AppError::ModelBindingDisabled {
                model: b.model_name.clone(),
            });
        }
        return Ok(b);
    }

    // 无 `/`：整段为分组别名，使用当前生效绑定
    let g = model_group_dao::get_by_alias_ci(conn, spec)
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::ModelNotBound {
            model: spec.to_string(),
        })?;
    if !g.is_enabled {
        return Err(AppError::ModelNotBound {
            model: spec.to_string(),
        });
    }
    let bid = g.active_binding_id.ok_or_else(|| {
        AppError::Internal(format!(
            "Model group '{}' has no active member selected. Please select an active member for this group before making requests.",
            g.alias
        ))
    })?;
    let b = model_binding_dao::get_by_id(conn, &bid)
        .map_err(AppError::Internal)?
        .ok_or_else(|| {
            AppError::Internal(
                "Group active member no longer exists. Please re-select in the console.".into(),
            )
        })?;
    if !b.is_enabled {
        return Err(AppError::ModelBindingDisabled {
            model: b.model_name.clone(),
        });
    }
    Ok(b)
}

pub fn list_group_members_by_alias(
    conn: &Connection,
    alias: &str,
) -> Result<Vec<RoutingMemberStatus>, AppError> {
    let group = model_group_dao::get_by_alias_ci(conn, alias)
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::Internal(format!("Model group '{alias}' not found.")))?;

    let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &group.id)
        .map_err(AppError::Internal)?;
    let mut members = Vec::new();
    for bid in binding_ids {
        let binding = model_binding_dao::get_by_id(conn, &bid)
            .map_err(AppError::Internal)?
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "Binding '{bid}' referenced by group '{}' no longer exists.",
                    group.alias
                ))
            })?;
        members.push(RoutingMemberStatus {
            binding_id: binding.id.clone(),
            name: binding.model_name.clone(),
            provider_id: binding.provider_id.clone(),
            upstream_model_name: binding.upstream_model_name.clone(),
            enabled: binding.is_enabled,
            active: group.active_binding_id.as_deref() == Some(binding.id.as_str()),
        });
    }
    Ok(members)
}

pub fn get_routing_status(conn: &Connection) -> Result<RoutingStatus, AppError> {
    let allow_group_member_model_path = load_gateway_config().allow_group_member_model_path;
    let groups = model_group_dao::list(conn).map_err(AppError::Internal)?;
    let mut out = Vec::new();

    for group in groups {
        let members = list_group_members_by_alias(conn, &group.alias)?;
        let active_member = members.iter().find(|m| m.active).map(|m| m.name.clone());
        out.push(RoutingGroupStatus {
            group_id: group.id,
            alias: group.alias,
            enabled: group.is_enabled,
            active_member,
            members,
        });
    }

    Ok(RoutingStatus {
        allow_group_member_model_path,
        groups: out,
    })
}

pub fn set_group_active_member_by_alias(
    conn: &Connection,
    group_alias: &str,
    member_name: &str,
) -> Result<RoutingGroupStatus, AppError> {
    let group = model_group_dao::get_by_alias_ci(conn, group_alias)
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::Internal(format!("Model group '{group_alias}' not found.")))?;

    let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &group.id)
        .map_err(AppError::Internal)?;

    let mut target_binding_id: Option<String> = None;
    for bid in binding_ids {
        let binding = model_binding_dao::get_by_id(conn, &bid)
            .map_err(AppError::Internal)?
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "Binding '{bid}' referenced by group '{}' no longer exists.",
                    group.alias
                ))
            })?;
        if binding.model_name.eq_ignore_ascii_case(member_name.trim()) {
            if target_binding_id.is_some() {
                return Err(AppError::Internal(format!(
                    "Group '{}' has more than one member named '{}'.",
                    group.alias, member_name
                )));
            }
            target_binding_id = Some(binding.id);
        }
    }

    let target_binding_id = target_binding_id.ok_or_else(|| {
        AppError::Internal(format!(
            "Group '{}' has no member named '{}'.",
            group.alias, member_name
        ))
    })?;

    let updated = model_group_dao::set_active_binding(conn, &group.id, Some(&target_binding_id))
        .map_err(AppError::Internal)?;
    let members = list_group_members_by_alias(conn, &updated.alias)?;
    let active_member = members.iter().find(|m| m.active).map(|m| m.name.clone());

    Ok(RoutingGroupStatus {
        group_id: updated.id,
        alias: updated.alias,
        enabled: updated.is_enabled,
        active_member,
        members,
    })
}
