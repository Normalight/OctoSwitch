use chrono::Utc;
use rusqlite::{params, Connection};

use crate::database::DaoError;
use crate::domain::copilot_account::CopilotAccount;

pub fn list(conn: &Connection) -> Result<Vec<CopilotAccount>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, github_user_id, github_login, avatar_url, github_token, copilot_token, token_expires_at, account_type, api_endpoint, created_at, updated_at FROM copilot_accounts ORDER BY id",
    )?;

    let iter = stmt.query_map([], row_to_account)?;

    let mut out = Vec::new();
    for row in iter {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_by_provider(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<CopilotAccount>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, github_user_id, github_login, avatar_url, github_token, copilot_token, token_expires_at, account_type, api_endpoint, created_at, updated_at FROM copilot_accounts WHERE provider_id = ?1",
    )?;

    let mut rows = stmt.query([provider_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_account(&row)?))
    } else {
        Ok(None)
    }
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<CopilotAccount>, DaoError> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, github_user_id, github_login, avatar_url, github_token, copilot_token, token_expires_at, account_type, api_endpoint, created_at, updated_at FROM copilot_accounts WHERE id = ?1",
    )?;

    let mut rows = stmt.query([id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_account(&row)?))
    } else {
        Ok(None)
    }
}

pub fn insert(conn: &Connection, account: &CopilotAccount) -> Result<i64, DaoError> {
    conn.execute(
        "INSERT INTO copilot_accounts (provider_id, github_user_id, github_login, avatar_url, github_token, copilot_token, token_expires_at, account_type, api_endpoint, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            account.provider_id,
            account.github_user_id,
            account.github_login,
            account.avatar_url,
            account.github_token,
            account.copilot_token,
            account.token_expires_at,
            account.account_type,
            account.api_endpoint,
            account.created_at,
            account.updated_at,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, account: &CopilotAccount) -> Result<(), DaoError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE copilot_accounts SET provider_id=?2, github_user_id=?3, github_login=?4, avatar_url=?5, github_token=?6, copilot_token=?7, token_expires_at=?8, account_type=?9, api_endpoint=?10, updated_at=?11 WHERE id=?1",
        params![
            account.id,
            account.provider_id,
            account.github_user_id,
            account.github_login,
            account.avatar_url,
            account.github_token,
            account.copilot_token,
            account.token_expires_at,
            account.account_type,
            account.api_endpoint,
            now,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), DaoError> {
    let n = conn.execute("DELETE FROM copilot_accounts WHERE id = ?1", [id])?;
    if n == 0 {
        return Err(DaoError::NotFound {
            entity: "copilot_account",
            id: id.to_string(),
        });
    }
    Ok(())
}

pub fn delete_by_provider(conn: &Connection, provider_id: &str) -> Result<(), DaoError> {
    conn.execute(
        "DELETE FROM copilot_accounts WHERE provider_id = ?1",
        [provider_id],
    )?;
    Ok(())
}

fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<CopilotAccount> {
    Ok(CopilotAccount {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        github_user_id: row.get(2).ok(),
        github_login: row.get(3)?,
        avatar_url: row.get(4).ok(),
        github_token: row.get(5).ok(),
        copilot_token: row.get(6).ok(),
        token_expires_at: row.get(7).ok(),
        account_type: row.get(8)?,
        api_endpoint: row.get(9).ok(),
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
