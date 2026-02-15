use ::clickhouse::Client;
use anyhow::Result;

use crate::db::clickhouse;
use crate::db::insert::upsert_exchange;

#[inline]
pub fn lower(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

pub async fn lookup_exchange_id_case_insensitive(
    _client: &Client,
    name: &str,
) -> Result<Option<(i32, String)>> {
    let canonical = name.trim();
    if canonical.is_empty() {
        return Ok(None);
    }

    let id = clickhouse::exchange_id(canonical);
    Ok(Some((id, canonical.to_string())))
}

pub async fn ensure_exchange_row(client: &Client, name: &str) -> Result<(i32, String)> {
    let canonical = name.trim();
    let id = upsert_exchange(client, canonical).await?;
    Ok((id, canonical.to_string()))
}
