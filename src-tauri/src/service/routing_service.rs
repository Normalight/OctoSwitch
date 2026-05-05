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
            ?
            .ok_or_else(|| AppError::ModelNotBound {
                model: spec.to_string(),
            })?;
        if !g.is_enabled {
            return Err(AppError::ModelNotBound {
                model: spec.to_string(),
            });
        }

        let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &g.id)
            ?;
        let mut matched: Option<ModelBinding> = None;
        for bid in binding_ids {
            let b = model_binding_dao::get_by_id(conn, &bid)
                ?
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
        ?
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
        ?
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
        ?
        .ok_or_else(|| AppError::Internal(format!("Model group '{alias}' not found.")))?;

    if !group.is_enabled {
        return Err(AppError::ModelGroupDisabled { alias: alias.to_string() });
    }

    let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &group.id)
        ?;
    let mut members = Vec::new();
    for bid in binding_ids {
        let binding = model_binding_dao::get_by_id(conn, &bid)
            ?
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
    let groups = model_group_dao::list(conn)?;
    let mut out = Vec::new();

    for group in groups {
        if !group.is_enabled {
            continue;
        }
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
        ?
        .ok_or_else(|| AppError::Internal(format!("Model group '{group_alias}' not found.")))?;

    if !group.is_enabled {
        return Err(AppError::ModelGroupDisabled { alias: group_alias.to_string() });
    }

    let binding_ids = model_group_member_dao::list_binding_ids_for_group(conn, &group.id)
        ?;

    let mut target_binding_id: Option<String> = None;
    for bid in binding_ids {
        let binding = model_binding_dao::get_by_id(conn, &bid)
            ?
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
        ?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        self, model_binding_dao, model_group_dao, model_group_member_dao, provider_dao,
    };
    use crate::domain::model_binding::NewModelBinding;
    use crate::domain::model_group::NewModelGroup;
    use crate::domain::provider::NewProvider;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        database::init_schema(&mut conn).expect("init schema");
        conn
    }

    fn create_test_provider(conn: &Connection, name: &str) -> String {
        let p = provider_dao::create(
            conn,
            NewProvider {
                name: name.to_string(),
                base_url: "https://api.example.com".to_string(),
                api_key_ref: "sk-test-key".to_string(),
                timeout_ms: 30000,
                max_retries: 2,
                is_enabled: true,
                api_format: Some("openai_chat".to_string()),
                auth_mode: "bearer".to_string(),
            },
        )
        .expect("create provider");
        p.id
    }

    fn create_test_binding(conn: &Connection, model_name: &str, provider_id: &str) -> String {
        let b = model_binding_dao::create(
            conn,
            NewModelBinding {
                model_name: model_name.to_string(),
                provider_id: provider_id.to_string(),
                upstream_model_name: "gpt-4".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: true,
            },
        )
        .expect("create binding");
        b.id
    }

    fn create_test_group(conn: &Connection, alias: &str) -> String {
        let g = model_group_dao::create(conn, NewModelGroup {
            alias: alias.to_string(),
        })
        .expect("create group");
        g.id
    }

    #[test]
    fn resolve_binding_by_group_alias() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid).expect("add member");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid)).expect("set active");

        let resolved = resolve_model_binding(&conn, "Sonnet", false).expect("resolve");
        assert_eq!(resolved.id, bid);
        assert_eq!(resolved.model_name, "my-gpt4");
        assert_eq!(resolved.provider_id, pid);
    }

    #[test]
    fn resolve_binding_by_member_path() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid).expect("add member");
        // Member path works regardless of active binding
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid)).expect("set active");

        let resolved = resolve_model_binding(&conn, "Sonnet/my-gpt4", true).expect("resolve");
        assert_eq!(resolved.id, bid);
        assert_eq!(resolved.model_name, "my-gpt4");
    }

    #[test]
    fn resolve_member_path_disabled_when_config_off() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid).expect("add member");

        let result = resolve_model_binding(&conn, "Sonnet/my-gpt4", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidModelSpec { subcode, .. } => {
                assert_eq!(subcode, crate::domain::model_spec_subcode::MEMBER_PATH_DISABLED);
            }
            other => panic!("Expected InvalidModelSpec with MEMBER_PATH_DISABLED, got {other:?}"),
        }
    }

    #[test]
    fn resolve_unknown_group_returns_model_not_bound() {
        let conn = setup_db();
        let result = resolve_model_binding(&conn, "Nonexistent", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ModelNotBound { model } => assert_eq!(model, "Nonexistent"),
            other => panic!("Expected ModelNotBound, got {other:?}"),
        }
    }

    #[test]
    fn resolve_disabled_binding_returns_error() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = model_binding_dao::create(
            &conn,
            NewModelBinding {
                model_name: "disabled-model".to_string(),
                provider_id: pid.clone(),
                upstream_model_name: "gpt-4".to_string(),
                rpm_limit: None,
                tpm_limit: None,
                is_enabled: false,
            },
        )
        .expect("create binding");
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid.id).expect("add member");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid.id)).expect("set active");

        let result = resolve_model_binding(&conn, "Sonnet", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ModelBindingDisabled { model } => {
                assert_eq!(model, "disabled-model");
            }
            other => panic!("Expected ModelBindingDisabled, got {other:?}"),
        }
    }

    #[test]
    fn resolve_disabled_group_returns_model_not_bound() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = model_group_dao::create(&conn, NewModelGroup {
            alias: "Offline".to_string(),
        })
        .expect("create group");

        model_group_member_dao::add(&conn, &gid.id, &bid).expect("add member");
        model_group_dao::set_active_binding(&conn, &gid.id, Some(&bid)).expect("set active");

        // Disable the group via update_partial
        model_group_dao::update_partial(
            &conn,
            &gid.id,
            serde_json::json!({"is_enabled": false}),
        )
        .expect("disable group");

        let result = resolve_model_binding(&conn, "Offline", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ModelNotBound { .. } => {}
            other => panic!("Expected ModelNotBound, got {other:?}"),
        }
    }

    #[test]
    fn resolve_group_without_active_member_returns_error() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid).expect("add member");
        // add() auto-sets first member as active; explicitly clear it for this test
        model_group_dao::set_active_binding(&conn, &gid, None).expect("clear active");

        let result = resolve_model_binding(&conn, "Sonnet", false);
        assert!(result.is_err());
    }

    #[test]
    fn list_group_members_returns_correct_data() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid1 = create_test_binding(&conn, "member-a", &pid);
        let bid2 = create_test_binding(&conn, "member-b", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid1).expect("add member a");
        model_group_member_dao::add(&conn, &gid, &bid2).expect("add member b");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid1)).expect("set active");

        let members = list_group_members_by_alias(&conn, "Sonnet").expect("list members");
        assert_eq!(members.len(), 2);

        let a = members.iter().find(|m| m.name == "member-a").expect("member-a exists");
        assert!(a.active);
        assert!(a.enabled);

        let b = members.iter().find(|m| m.name == "member-b").expect("member-b exists");
        assert!(!b.active);
        assert!(b.enabled);
    }

    #[test]
    fn set_active_member_switches_correctly() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid1 = create_test_binding(&conn, "member-a", &pid);
        let bid2 = create_test_binding(&conn, "member-b", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid1).expect("add member a");
        model_group_member_dao::add(&conn, &gid, &bid2).expect("add member b");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid1)).expect("set active");

        // Switch to member-b
        let group_status =
            set_group_active_member_by_alias(&conn, "Sonnet", "member-b")
                .expect("switch active");
        assert_eq!(group_status.active_member, Some("member-b".to_string()));

        // Verify routing resolves to member-b
        let resolved = resolve_model_binding(&conn, "Sonnet", false).expect("resolve");
        assert_eq!(resolved.model_name, "member-b");
    }

    #[test]
    fn set_active_member_nonexistent_member_returns_error() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid1 = create_test_binding(&conn, "member-a", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid1).expect("add member");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid1)).expect("set active");

        let result =
            set_group_active_member_by_alias(&conn, "Sonnet", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn routing_status_includes_groups() {
        let conn = setup_db();
        let pid = create_test_provider(&conn, "TestProvider");
        let bid = create_test_binding(&conn, "my-gpt4", &pid);
        let gid = create_test_group(&conn, "Sonnet");

        model_group_member_dao::add(&conn, &gid, &bid).expect("add member");
        model_group_dao::set_active_binding(&conn, &gid, Some(&bid)).expect("set active");

        let status = get_routing_status(&conn).expect("get status");
        assert_eq!(status.groups.len(), 1);
        assert_eq!(status.groups[0].alias, "Sonnet");
        assert_eq!(status.groups[0].active_member, Some("my-gpt4".to_string()));
        assert_eq!(status.groups[0].members.len(), 1);
    }

    #[test]
    fn empty_model_spec_returns_error() {
        let conn = setup_db();
        let result = resolve_model_binding(&conn, "", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidModelSpec { subcode, .. } => {
                assert_eq!(
                    subcode,
                    crate::domain::model_spec_subcode::MODEL_SPEC_EMPTY
                );
            }
            other => panic!("Expected InvalidModelSpec, got {other:?}"),
        }
    }

    #[test]
    fn slash_with_empty_parts_returns_error() {
        let conn = setup_db();
        let result = resolve_model_binding(&conn, "/foo", true);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidModelSpec { subcode, .. } => {
                assert_eq!(
                    subcode,
                    crate::domain::model_spec_subcode::GROUP_MEMBER_EMPTY_PART
                );
            }
            other => panic!("Expected InvalidModelSpec, got {other:?}"),
        }
    }

    #[test]
    fn routing_name_with_slash_returns_error() {
        let conn = setup_db();
        let result = resolve_model_binding(&conn, "group/a/b", true);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidModelSpec { subcode, .. } => {
                assert_eq!(
                    subcode,
                    crate::domain::model_spec_subcode::ROUTING_NAME_CONTAINS_SLASH
                );
            }
            other => panic!("Expected InvalidModelSpec, got {other:?}"),
        }
    }

    #[test]
    fn nonexistent_group_in_member_path_returns_model_not_bound() {
        let conn = setup_db();
        let result = resolve_model_binding(&conn, "FakeGroup/some-binding", true);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ModelNotBound { .. } => {}
            other => panic!("Expected ModelNotBound, got {other:?}"),
        }
    }

    #[test]
    fn group_without_members_returns_error_on_resolve() {
        let conn = setup_db();
        let _gid = create_test_group(&conn, "EmptyGroup");
        // Group has no members and no active binding

        let result = resolve_model_binding(&conn, "EmptyGroup", false);
        assert!(result.is_err());
    }
}
