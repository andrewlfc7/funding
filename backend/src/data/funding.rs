// src/data/funding.rs
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use tracing::{info, warn};

use futures::{stream, StreamExt, TryStreamExt};

use crate::db::insert::insert_funding_rates;
use crate::exchanges::shared::types::NormalizedFundingRate;

use crate::exchanges::extended::api::{client::ExtendedClient, endpoints::ApiEnvironment as ExtendedEnv};
use crate::exchanges::extended::handler::handler::parse_extended_funding;
use crate::exchanges::paradex::api::{client::ParadexClient, endpoints::ApiEnvironment as ParadexEnv};
use crate::exchanges::paradex::handler::handler::parse_paradex_funding;

#[inline]
fn lower(s: &str) -> String { s.trim().to_ascii_lowercase() }


#[derive(Clone, Debug)]
pub enum TimeSpec {
    Between { start_ms: u64, end_ms: u64 },
    LookbackHours(u64),
    SinceLastOrLookbackHours(u64),
}

impl TimeSpec {
    fn resolve(&self, last_ts_ms: Option<i64>) -> (u64, u64) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        match *self {
            TimeSpec::Between { start_ms, end_ms } => (start_ms, end_ms),
            TimeSpec::LookbackHours(h) => (now_ms.saturating_sub(h.saturating_mul(3_600_000)), now_ms),
            TimeSpec::SinceLastOrLookbackHours(h) => {
                if let Some(last_ms) = last_ts_ms {
                    ((last_ms as u64).saturating_add(1), now_ms)
                } else {
                    (now_ms.saturating_sub(h.saturating_mul(3_600_000)), now_ms)
                }
            }
        }
    }
}


async fn fetch_funding_for_market(
    exchange_name: &str,
    market_symbol: &str,
    start_ms: u64,
    end_ms: u64,
) -> Result<Vec<NormalizedFundingRate>> {
    match lower(exchange_name).as_str() {
        "paradex" => {
            let client = ParadexClient::new(ParadexEnv::Mainnet);
            let raw = client
                .get_funding_data(market_symbol, Some(start_ms), Some(end_ms))
                .await?;
            Ok(parse_paradex_funding(&raw)?)
        }
        "extended" => {
            let client = ExtendedClient::new(ExtendedEnv::Mainnet);
            let raw = client
                .get_funding(market_symbol, Some(start_ms), Some(end_ms))
                .await?;
            Ok(parse_extended_funding(&raw)?)
        }
        other => {
            warn!("fetch_funding_for_market: unsupported exchange '{}'", other);
            Ok(Vec::new())
        }
    }
}


pub async fn collect_funding_for_exchange_with_spec(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
    time_spec: TimeSpec,
) -> Result<()> {
    // NEW: force interval to 8h (480 minutes), idempotent
    let res = sqlx::query!(
        r#"
        UPDATE exchanges
        SET funding_interval_minutes = 480, updated_at = NOW()
        WHERE id = $1 AND (funding_interval_minutes IS NULL OR funding_interval_minutes <> 480)
        "#,
        exchange_id
    )
    .execute(pool)
    .await?;
    if res.rows_affected() > 0 {
        info!("funding interval for {} (id={}) set to 480m", exchange_name, exchange_id);
    }

    // 1) markets
    let markets = sqlx::query!(
        r#"
        SELECT id, market_symbol
        FROM markets
        WHERE exchange_id = $1 AND is_active = true
        ORDER BY market_symbol
        "#,
        exchange_id
    )
    .fetch_all(pool)
    .await?;

    if markets.is_empty() {
        info!("no active markets for {}", exchange_name);
        return Ok(());
    }

    
    let conc: usize = std::env::var("SYNC_CONC_MARKETS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);

    let per_market_batches: Vec<Vec<(i32, NormalizedFundingRate)>> =
        stream::iter(markets.into_iter())
            .map(|m| {
                let pool = pool.clone();
                let exchange_name = exchange_name.to_string();
                let time_spec = time_spec.clone();

                async move {
                    let last_ts_ms_opt = sqlx::query!(
                        r#"SELECT MAX(timestamp) AS last_ts FROM funding_rates WHERE market_id = $1"#,
                        m.id
                    )
                    .fetch_optional(&pool)
                    .await?
                    .and_then(|r| r.last_ts.map(|odt| odt.unix_timestamp() * 1000));

                    let (start_ms, end_ms) = time_spec.resolve(last_ts_ms_opt);
                    if start_ms > end_ms {
                        warn!(
                            "skip {} {}: invalid window start_ms={} > end_ms={}",
                            exchange_name, m.market_symbol, start_ms, end_ms
                        );
                        return Ok::<Vec<(i32, NormalizedFundingRate)>, anyhow::Error>(Vec::new());
                    }

                    let rows = fetch_funding_for_market(&exchange_name, &m.market_symbol, start_ms, end_ms).await?;
                    let out: Vec<(i32, NormalizedFundingRate)> = rows.into_iter().map(|r| (m.id, r)).collect();
                    Ok::<Vec<(i32, NormalizedFundingRate)>, anyhow::Error>(out)
                }
            })
            .buffer_unordered(conc)
            .try_collect::<Vec<Vec<(i32, NormalizedFundingRate)>>>()
            .await?;

    
    let owned: Vec<(i32, NormalizedFundingRate)> =
        per_market_batches.into_iter().flat_map(|v| v.into_iter()).collect();

    if owned.is_empty() {
        info!("no funding rows to insert for {}", exchange_name);
        return Ok(());
    }

    let borrowed: Vec<(i32, &NormalizedFundingRate)> =
        owned.iter().map(|(mid, r)| (*mid, r)).collect();

    insert_funding_rates(pool, exchange_id, &borrowed).await?;
    info!("inserted {} funding rows for {}", borrowed.len(), exchange_name);

    Ok(())
}


pub async fn collect_funding_for_exchange(
    pool: &PgPool,
    exchange_id: i32,
    exchange_name: &str,
) -> Result<()> {
    collect_funding_for_exchange_with_spec(
        pool,
        exchange_id,
        exchange_name,
        TimeSpec::SinceLastOrLookbackHours(24),
    )
    .await
}
