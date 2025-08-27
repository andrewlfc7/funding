// src/data/funding.rs
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::db::insert::insert_funding_rates;
use crate::exchanges::extended::api::{
    client::ExtendedClient,
    endpoints::ApiEnvironment as ExtendedEnv,
};
use crate::exchanges::extended::handler::handler::parse_extended_funding;
use crate::exchanges::paradex::api::{
    client::ParadexClient,
    endpoints::ApiEnvironment as ParadexEnv,
};
use crate::exchanges::paradex::handler::handler::parse_paradex_funding;



pub async fn collect_funding_for_exchange(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
) -> Result<()> {
    let markets = sqlx::query!(
        r#"SELECT id, market_symbol
           FROM markets
           WHERE exchange_id = $1 AND is_active = true"#,
        exchange_id
    )
    .fetch_all(pool)
    .await?;

    for m in markets {
        let row = sqlx::query!(
            r#"SELECT MAX(timestamp) AS last_ts
               FROM funding_rates
               WHERE market_id = $1"#,
            m.id
        )
        .fetch_optional(pool)
        .await?;

        let start_ms = row
            .and_then(|r| r.last_ts)
            .map(|t| (t.unix_timestamp() as i128 * 1000) as u64)
            .unwrap_or_else(|| (Utc::now().timestamp_millis() as u64).saturating_sub(86_400_000));

        let end_ms = Utc::now().timestamp_millis() as u64;

        match exchange_name {
            "paradex" => {
                let client = ParadexClient::new(ParadexEnv::Mainnet);
                let raw = client
                    .get_funding_data(&m.market_symbol, Some(start_ms), Some(end_ms),None)
                    .await?;
                let rates = parse_paradex_funding(&raw)?;
                if !rates.is_empty() {
                    insert_funding_rates(pool, exchange_id, m.id, &rates).await?;
                }
            }
            "extended" => {
                let client = ExtendedClient::new(ExtendedEnv::Mainnet);
                let raw = client
                    .get_funding(&m.market_symbol, Some(start_ms), Some(end_ms))
                    .await?;
                let rates = parse_extended_funding(&raw)?;
                if !rates.is_empty() {
                    insert_funding_rates(pool, exchange_id, m.id, &rates).await?;
                }
            }
            _ => {
                // unknown/unsupported exchange name; skip
            }
        }
    }

    Ok(())
}
