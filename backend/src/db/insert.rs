use anyhow::Result;
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use time::OffsetDateTime;
use std::str::FromStr;

use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats,
};

pub async fn upsert_markets(
    pool: &PgPool,
    exchange_id: i32,
    markets: &[NormalizedMarket]
) -> Result<()> {
    for m in markets {
        let token_id: i32 = sqlx::query_scalar!(
            "INSERT INTO tokens (symbol) VALUES ($1)
             ON CONFLICT(symbol) DO UPDATE SET symbol = EXCLUDED.symbol
             RETURNING id",
            m.symbol
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO markets (exchange_id, token_id, market_symbol, is_active)
             VALUES ($1,$2,$3,$4)
             ON CONFLICT(exchange_id, market_symbol)
             DO UPDATE SET is_active = EXCLUDED.is_active",
            exchange_id,
            token_id,
            m.market_symbol,
            m.is_active
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn insert_funding_rates(
    pool: &PgPool,
    exchange_id: i32,
    market_id: i32,
    rates: &[NormalizedFundingRate]
) -> Result<()> {
    for r in rates {
        let rate = BigDecimal::from_str(&r.rate.to_string())?;
        let timestamp = OffsetDateTime::from_unix_timestamp(r.timestamp.timestamp())?;

        sqlx::query!(
            "INSERT INTO funding_rates (exchange_id, market_id, rate, timestamp)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (market_id, timestamp) DO NOTHING",
            exchange_id,
            market_id,
            rate,
            timestamp
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn insert_market_stats(
    pool: &PgPool,
    market_id: i32,
    stat: &NormalizedMarketStats
) -> Result<()> {
    let open_interest = stat.open_interest
        .as_ref()
        .map(|d| BigDecimal::from_str(&d.to_string()))
        .transpose()?;

    let volume_24h = stat.volume_24h
        .as_ref()
        .map(|d| BigDecimal::from_str(&d.to_string()))
        .transpose()?;

    let timestamp = OffsetDateTime::from_unix_timestamp(stat.timestamp.timestamp())?;

    sqlx::query!(
        "INSERT INTO market_stats (market_id, open_interest, volume_24h, timestamp)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (market_id, timestamp) DO NOTHING",
        market_id,
        open_interest,
        volume_24h,
        timestamp
    )
    .execute(pool)
    .await?;
    Ok(())
}
