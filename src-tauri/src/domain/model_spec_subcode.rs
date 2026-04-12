//! Stable `subcode` values for JSON error bodies when `code` is `INVALID_MODEL_SPEC`.
//! Clients and docs can rely on these identifiers; the `error` string is English.

pub const MODEL_SPEC_EMPTY: &str = "MODEL_SPEC_EMPTY";
pub const MEMBER_PATH_DISABLED: &str = "MEMBER_PATH_DISABLED";
pub const GROUP_MEMBER_EMPTY_PART: &str = "GROUP_MEMBER_EMPTY_PART";
pub const ROUTING_NAME_CONTAINS_SLASH: &str = "ROUTING_NAME_CONTAINS_SLASH";
pub const DUPLICATE_ROUTING_NAME_IN_GROUP: &str = "DUPLICATE_ROUTING_NAME_IN_GROUP";
