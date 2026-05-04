use rusqlite::Connection;

use crate::{database::model_group_dao, domain::model_group::NewModelGroup};

pub const DEFAULT_GROUP_ALIASES: &[&str] = &["Sonnet", "Opus", "Haiku"];

pub fn ensure_default_model_groups(conn: &Connection) -> Result<(), String> {
    let existing = model_group_dao::list(conn).map_err(|e| e.to_string())?;
    if !existing.is_empty() {
        return Ok(());
    }

    for alias in DEFAULT_GROUP_ALIASES {
        model_group_dao::create(
            conn,
            NewModelGroup {
                alias: (*alias).to_string(),
            },
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn reset_with_default_model_groups(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    crate::database::clear_all_data(&tx)?;

    let existing = model_group_dao::list(&tx).map_err(|e| e.to_string())?;
    if existing.is_empty() {
        for alias in DEFAULT_GROUP_ALIASES {
            model_group_dao::create(
                &tx,
                NewModelGroup {
                    alias: (*alias).to_string(),
                },
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_default_model_groups, reset_with_default_model_groups, DEFAULT_GROUP_ALIASES,
    };
    use crate::{database, database::model_group_dao, domain::model_group::NewModelGroup};
    use rusqlite::Connection;

    #[test]
    fn seeds_default_groups_when_empty() {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        database::init_schema(&mut conn).expect("init schema");

        ensure_default_model_groups(&conn).expect("seed defaults");

        let groups = model_group_dao::list(&conn).expect("list groups");
        let aliases: Vec<String> = groups.into_iter().map(|g| g.alias).collect();
        let expected: Vec<String> = DEFAULT_GROUP_ALIASES
            .iter()
            .map(|alias| (*alias).to_string())
            .collect();

        assert_eq!(aliases, expected);
    }

    #[test]
    fn does_not_seed_when_groups_already_exist() {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        database::init_schema(&mut conn).expect("init schema");
        model_group_dao::create(
            &conn,
            NewModelGroup {
                alias: "Custom".to_string(),
            },
        )
        .expect("create custom group");

        ensure_default_model_groups(&conn).expect("skip seeding");

        let groups = model_group_dao::list(&conn).expect("list groups");
        let aliases: Vec<String> = groups.into_iter().map(|g| g.alias).collect();

        assert_eq!(aliases, vec!["Custom".to_string()]);
    }

    #[test]
    fn reset_recreates_default_groups_transactionally() {
        let mut conn = Connection::open_in_memory().expect("open memory db");
        database::init_schema(&mut conn).expect("init schema");
        model_group_dao::create(
            &conn,
            NewModelGroup {
                alias: "Custom".to_string(),
            },
        )
        .expect("create custom group");

        reset_with_default_model_groups(&mut conn).expect("reset with defaults");

        let groups = model_group_dao::list(&conn).expect("list groups");
        let aliases: Vec<String> = groups.into_iter().map(|g| g.alias).collect();
        let expected: Vec<String> = DEFAULT_GROUP_ALIASES
            .iter()
            .map(|alias| (*alias).to_string())
            .collect();

        assert_eq!(aliases, expected);
    }
}
