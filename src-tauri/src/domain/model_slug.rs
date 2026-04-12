//! 客户端 `model` 仅允许 `分组别名` 或 `分组别名/绑定路由名`；段内不得含 `/`。

/// 校验名称中不含 `/`，用于分组别名与绑定路由名。
pub fn validate_no_slash(name: &str, field_label: &str) -> Result<(), String> {
    if name.contains('/') {
        return Err(format!("{field_label}不能包含字符 /"));
    }
    Ok(())
}
