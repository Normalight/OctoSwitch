//! 客户端 `model` 仅允许 `分组别名` 或 `分组别名/绑定路由名`；段内不得含 `/`。

/// 校验名称中不含 `/`，用于分组别名与绑定路由名。
pub fn validate_no_slash(name: &str, field_label: &str) -> Result<(), String> {
    if name.contains('/') {
        return Err(format!("{field_label}不能包含字符 /"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_name_with_slash() {
        assert!(validate_no_slash("foo/bar", "alias").is_err());
    }

    #[test]
    fn accepts_plain_name() {
        assert!(validate_no_slash("my-model", "alias").is_ok());
    }

    #[test]
    fn accepts_name_with_hyphens_and_underscores() {
        assert!(validate_no_slash("gpt-4_turbo", "model_name").is_ok());
    }

    #[test]
    fn accepts_empty_name() {
        assert!(validate_no_slash("", "alias").is_ok());
    }

    #[test]
    fn error_message_contains_field_label() {
        let err = validate_no_slash("a/b", "分组别名").unwrap_err();
        assert!(err.contains("分组别名"));
    }
}
