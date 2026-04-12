use rusqlite::Connection;

use crate::database::{model_binding_dao, model_group_dao, model_group_member_dao};
use crate::domain::error::AppError;
use crate::domain::model_spec_subcode::{
    DUPLICATE_ROUTING_NAME_IN_GROUP, GROUP_MEMBER_EMPTY_PART, MEMBER_PATH_DISABLED,
    MODEL_SPEC_EMPTY, ROUTING_NAME_CONTAINS_SLASH,
};
use crate::domain::model_binding::ModelBinding;
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
        .ok_or_else(|| AppError::Internal("Group active member no longer exists. Please re-select in the console.".into()))?;
    if !b.is_enabled {
        return Err(AppError::ModelBindingDisabled {
            model: b.model_name.clone(),
        });
    }
    Ok(b)
}
